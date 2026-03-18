use {
	crate::plugin::GlorpPlugin,
	glorp_api::{
		GlorpError, GlorpEventStreamView, GlorpHost, GlorpOutcome, GlorpQueryResult, GlorpSessionView, GlorpValue,
		HelperKind, SurfaceArgKind, SurfaceKind, SurfaceSpec, query_invocation, surface_specs,
	},
	glorp_transport::{
		IpcClient, TransportRequest, TransportResponse, default_socket_path, socket_is_live, transport_request,
	},
	nu_plugin::{EvaluatedCall, PluginCommand},
	nu_protocol::{Category, LabeledError, PipelineData, Record, Signature, Span, SyntaxShape, Type, Value},
	serde_json::Value as JsonValue,
	std::{
		path::{Path, PathBuf},
		process::{Command, Stdio},
		thread,
		time::{Duration, Instant},
	},
};

pub fn all_commands() -> Vec<Box<dyn PluginCommand<Plugin = GlorpPlugin>>> {
	surface_specs()
		.into_iter()
		.map(|spec| Box::new(SurfaceCommand { spec }) as Box<dyn PluginCommand<Plugin = GlorpPlugin>>)
		.collect()
}

struct SurfaceCommand {
	spec: SurfaceSpec,
}

impl PluginCommand for SurfaceCommand {
	type Plugin = GlorpPlugin;

	fn name(&self) -> &str {
		self.spec.path
	}

	fn description(&self) -> &str {
		self.spec.docs
	}

	fn signature(&self) -> Signature {
		let mut signature = Signature::build(self.spec.path)
			.named("socket", SyntaxShape::String, "Runtime Unix socket path.", Some('s'))
			.named("session", SyntaxShape::Any, "Resolved Glorp session record.", None)
			.named(
				"repo-root",
				SyntaxShape::String,
				"Repo root for shared runtime discovery.",
				Some('r'),
			)
			.input_output_types(vec![(Type::Nothing, Type::Any)])
			.category(Category::Custom("glorp".to_owned()));

		for arg in &self.spec.args {
			let shape = syntax_shape(arg.kind);
			signature = if arg.required {
				signature.required(arg.name, shape, arg.docs)
			} else {
				signature.optional(arg.name, shape, arg.docs)
			};
		}

		signature
	}

	fn run(
		&self, _plugin: &Self::Plugin, _engine: &nu_plugin::EngineInterface, call: &EvaluatedCall, _input: PipelineData,
	) -> Result<PipelineData, LabeledError> {
		let span = call.head;
		let input = call_input(call, &self.spec)?;
		match self.spec.kind {
			SurfaceKind::Command => {
				let mut client = resolve_session(call)?.client;
				let outcome = client
					.execute(glorp_api::command_invocation(self.spec.path, input.as_ref()).map_err(to_labeled_error)?)
					.map_err(to_labeled_error)?;
				Ok(value_pipeline(outcome_to_value(outcome, span)))
			}
			SurfaceKind::Query => {
				let mut client = resolve_session(call)?.client;
				let result = client
					.query(query_invocation(self.spec.path, input.as_ref()).map_err(to_labeled_error)?)
					.map_err(to_labeled_error)?;
				Ok(value_pipeline(query_to_value(result, span)))
			}
			SurfaceKind::Helper(kind) => run_helper(kind, call, input.as_ref(), span),
		}
	}
}

fn run_helper(
	kind: HelperKind, call: &EvaluatedCall, input: Option<&GlorpValue>, span: Span,
) -> Result<PipelineData, LabeledError> {
	match kind {
		HelperKind::ConfigValidate => {
			let mut fields = input
				.and_then(GlorpValue::as_record)
				.ok_or_else(|| LabeledError::new("config validate requires record input"))?
				.iter();
			let path = fields
				.find_map(|(key, value)| (key == "path").then_some(value))
				.and_then(GlorpValue::as_str)
				.ok_or_else(|| LabeledError::new("config validate path must be a string"))?;
			let value = input
				.and_then(GlorpValue::as_record)
				.and_then(|fields| fields.get("value"))
				.cloned()
				.ok_or_else(|| LabeledError::new("config validate value is required"))?;
			glorp_api::GlorpConfig::validate_path(path, value).map_err(to_labeled_error)?;
			Ok(value_pipeline(json_to_nu_value(
				serde_json::json!({ "ok": true }),
				span,
			)))
		}
		HelperKind::SessionAttach => {
			let mut resolved = resolve_session(call)?;
			let capabilities = capabilities(&mut resolved.client)?;
			Ok(value_pipeline(json_to_nu_value(
				serde_json::to_value(GlorpSessionView {
					socket: resolved.socket.display().to_string(),
					repo_root: Some(resolved.repo_root.display().to_string()),
					capabilities,
				})
				.unwrap_or(JsonValue::Null),
				span,
			)))
		}
		HelperKind::SessionShutdown => {
			let resolved = resolve_session(call)?;
			let TransportResponse::Shutdown(result) =
				transport_request(&resolved.socket, &TransportRequest::Shutdown).map_err(to_labeled_error)?
			else {
				return Err(LabeledError::new("unexpected shutdown response"));
			};
			result.map_err(to_labeled_error)?;
			Ok(value_pipeline(json_to_nu_value(
				serde_json::json!({ "ok": true }),
				span,
			)))
		}
		HelperKind::EventsSubscribe => {
			let mut client = resolve_session(call)?.client;
			let token = client
				.subscribe(glorp_api::GlorpSubscription::Changes)
				.map_err(to_labeled_error)?;
			Ok(value_pipeline(json_to_nu_value(
				serde_json::to_value(GlorpEventStreamView {
					token,
					subscription: "changes".to_owned(),
				})
				.unwrap_or(JsonValue::Null),
				span,
			)))
		}
		HelperKind::EventsNext => {
			let token = token_from_input(input)?;
			let mut client = resolve_session(call)?.client;
			let event = client.next_event(token).map_err(to_labeled_error)?;
			Ok(value_pipeline(json_to_nu_value(
				serde_json::to_value(event).unwrap_or(JsonValue::Null),
				span,
			)))
		}
		HelperKind::EventsUnsubscribe => {
			let token = token_from_input(input)?;
			let mut client = resolve_session(call)?.client;
			client.unsubscribe(token).map_err(to_labeled_error)?;
			Ok(value_pipeline(json_to_nu_value(
				serde_json::json!({ "ok": true, "token": token }),
				span,
			)))
		}
	}
}

struct ResolvedSession {
	repo_root: PathBuf,
	socket: PathBuf,
	client: IpcClient,
}

fn resolve_session(call: &EvaluatedCall) -> Result<ResolvedSession, LabeledError> {
	if let Some(session) = call.get_flag::<Value>("session")? {
		let socket = session_socket(&session)?;
		let repo_root = session_repo_root(&session)?.unwrap_or_else(repo_root_from_call);
		return ensure_session(repo_root, socket);
	}

	if let Some(socket) = call.get_flag::<String>("socket")? {
		return ensure_session(repo_root_from_call(), PathBuf::from(socket));
	}

	let repo_root = call
		.get_flag::<String>("repo-root")?
		.map(PathBuf::from)
		.unwrap_or_else(repo_root_from_call);
	let socket = default_socket_path(&repo_root);
	ensure_session(repo_root, socket)
}

fn ensure_session(repo_root: PathBuf, socket: PathBuf) -> Result<ResolvedSession, LabeledError> {
	if !socket_is_live(&socket) {
		spawn_host(&repo_root, &socket)?;
		wait_for_socket(&socket)?;
	}

	let mut client = IpcClient::new(socket.clone());
	let _ = capabilities(&mut client)?;
	Ok(ResolvedSession {
		repo_root,
		socket,
		client,
	})
}

fn capabilities(client: &mut impl GlorpHost) -> Result<glorp_api::GlorpCapabilities, LabeledError> {
	let GlorpQueryResult::Capabilities(capabilities) = client
		.query(glorp_api::GlorpQuery::Capabilities)
		.map_err(to_labeled_error)?
	else {
		return Err(LabeledError::new("unexpected capabilities response"));
	};

	Ok(capabilities)
}

fn spawn_host(repo_root: &Path, socket: &Path) -> Result<(), LabeledError> {
	let host_bin = host_binary_path()?;
	Command::new(host_bin)
		.arg("--repo-root")
		.arg(repo_root)
		.arg("--socket")
		.arg(socket)
		.stdin(Stdio::null())
		.stdout(Stdio::null())
		.stderr(Stdio::null())
		.spawn()
		.map_err(|error| LabeledError::new(format!("failed to spawn glorp_host: {error}")))?;
	Ok(())
}

fn host_binary_path() -> Result<PathBuf, LabeledError> {
	if let Some(path) = std::env::var_os("GLORP_HOST_BIN") {
		return Ok(PathBuf::from(path));
	}

	if let Ok(current) = std::env::current_exe() {
		let sibling = current.with_file_name("glorp_host");
		if sibling.exists() {
			return Ok(sibling);
		}
	}

	Ok(PathBuf::from("glorp_host"))
}

fn wait_for_socket(socket: &Path) -> Result<(), LabeledError> {
	let deadline = Instant::now() + Duration::from_secs(5);
	while Instant::now() < deadline {
		if socket_is_live(socket) {
			return Ok(());
		}
		thread::sleep(Duration::from_millis(25));
	}

	Err(LabeledError::new(format!(
		"timed out waiting for live runtime at {}",
		socket.display()
	)))
}

fn repo_root_from_call() -> PathBuf {
	std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn call_input(call: &EvaluatedCall, spec: &SurfaceSpec) -> Result<Option<GlorpValue>, LabeledError> {
	if spec.args.is_empty() {
		return Ok(None);
	}

	let mut fields = std::collections::BTreeMap::new();
	for (index, arg) in spec.args.iter().enumerate() {
		match arg.kind {
			SurfaceArgKind::String => {
				if arg.required {
					let value: String = call.req(index)?;
					fields.insert(arg.name.to_owned(), GlorpValue::String(value));
				} else if let Some(value) = call.opt::<String>(index)? {
					fields.insert(arg.name.to_owned(), GlorpValue::String(value));
				}
			}
			SurfaceArgKind::Int => {
				if arg.required {
					let value: i64 = call.req(index)?;
					fields.insert(arg.name.to_owned(), GlorpValue::Int(value));
				} else if let Some(value) = call.opt::<i64>(index)? {
					fields.insert(arg.name.to_owned(), GlorpValue::Int(value));
				}
			}
			SurfaceArgKind::Float => {
				if arg.required {
					let value: f64 = call.req(index)?;
					fields.insert(arg.name.to_owned(), GlorpValue::Float(value));
				} else if let Some(value) = call.opt::<f64>(index)? {
					fields.insert(arg.name.to_owned(), GlorpValue::Float(value));
				}
			}
			SurfaceArgKind::Bool => {
				if arg.required {
					let value: bool = call.req(index)?;
					fields.insert(arg.name.to_owned(), GlorpValue::Bool(value));
				} else if let Some(value) = call.opt::<bool>(index)? {
					fields.insert(arg.name.to_owned(), GlorpValue::Bool(value));
				}
			}
			SurfaceArgKind::Any => {
				if arg.required {
					let value: Value = call.req(index)?;
					fields.insert(arg.name.to_owned(), glorp_value(value)?);
				} else if let Some(value) = call.opt::<Value>(index)? {
					fields.insert(arg.name.to_owned(), glorp_value(value)?);
				}
			}
		}
	}

	Ok(Some(GlorpValue::Record(fields)))
}

fn token_from_input(input: Option<&GlorpValue>) -> Result<u64, LabeledError> {
	let Some(GlorpValue::Record(fields)) = input else {
		return Err(LabeledError::new("subscription token input must be a record"));
	};
	let token = fields
		.get("token")
		.and_then(GlorpValue::as_i64)
		.ok_or_else(|| LabeledError::new("subscription token must be an integer"))?;
	u64::try_from(token).map_err(|_| LabeledError::new("subscription token must be non-negative"))
}

fn session_socket(value: &Value) -> Result<PathBuf, LabeledError> {
	let Value::Record { val, .. } = value else {
		return Err(LabeledError::new(
			"session flag must be a record returned by glorp session attach",
		));
	};

	match val.get("socket") {
		Some(Value::String { val, .. }) => Ok(PathBuf::from(val)),
		_ => Err(LabeledError::new(
			"session record does not contain a string `socket` field",
		)),
	}
}

fn session_repo_root(value: &Value) -> Result<Option<PathBuf>, LabeledError> {
	let Value::Record { val, .. } = value else {
		return Err(LabeledError::new(
			"session flag must be a record returned by glorp session attach",
		));
	};

	Ok(match val.get("repo_root") {
		Some(Value::String { val, .. }) => Some(PathBuf::from(val)),
		Some(Value::Nothing { .. }) | None => None,
		_ => {
			return Err(LabeledError::new(
				"session record contains a non-string `repo_root` field",
			));
		}
	})
}

fn syntax_shape(kind: SurfaceArgKind) -> SyntaxShape {
	match kind {
		SurfaceArgKind::String => SyntaxShape::String,
		SurfaceArgKind::Int => SyntaxShape::Int,
		SurfaceArgKind::Float => SyntaxShape::Number,
		SurfaceArgKind::Bool => SyntaxShape::Any,
		SurfaceArgKind::Any => SyntaxShape::Any,
	}
}

const fn value_pipeline(value: Value) -> PipelineData {
	PipelineData::Value(value, None)
}

fn outcome_to_value(outcome: GlorpOutcome, span: Span) -> Value {
	json_to_nu_value(serde_json::to_value(outcome).unwrap_or(JsonValue::Null), span)
}

fn query_to_value(result: GlorpQueryResult, span: Span) -> Value {
	let value = match result {
		GlorpQueryResult::Schema(value) => serde_json::to_value(value),
		GlorpQueryResult::Config(value) => serde_json::to_value(value),
		GlorpQueryResult::Snapshot(value) => serde_json::to_value(value),
		GlorpQueryResult::DocumentText(value) => serde_json::to_value(value),
		GlorpQueryResult::Selection(value) => serde_json::to_value(value),
		GlorpQueryResult::InspectDetails(value) => serde_json::to_value(value),
		GlorpQueryResult::PerfDashboard(value) => serde_json::to_value(value),
		GlorpQueryResult::UiState(value) => serde_json::to_value(value),
		GlorpQueryResult::Capabilities(value) => serde_json::to_value(value),
	}
	.unwrap_or(JsonValue::Null);
	json_to_nu_value(value, span)
}

fn glorp_value(value: Value) -> Result<GlorpValue, LabeledError> {
	match value {
		Value::Nothing { .. } => Ok(GlorpValue::Null),
		Value::Bool { val, .. } => Ok(GlorpValue::Bool(val)),
		Value::Int { val, .. } => Ok(GlorpValue::Int(val)),
		Value::Float { val, .. } => Ok(GlorpValue::Float(val)),
		Value::String { val, .. } | Value::Glob { val, .. } => Ok(GlorpValue::String(val)),
		Value::List { vals, .. } => vals
			.into_iter()
			.map(glorp_value)
			.collect::<Result<Vec<_>, _>>()
			.map(GlorpValue::List),
		Value::Record { val, .. } => val
			.into_owned()
			.into_iter()
			.map(|(key, value)| glorp_value(value).map(|value| (key, value)))
			.collect::<Result<_, _>>()
			.map(GlorpValue::Record),
		other => Err(LabeledError::new(format!(
			"unsupported value type `{}`",
			other.get_type()
		))),
	}
}

fn json_to_nu_value(value: JsonValue, span: Span) -> Value {
	match value {
		JsonValue::Null => Value::nothing(span),
		JsonValue::Bool(value) => Value::bool(value, span),
		JsonValue::Number(number) => number.as_i64().map_or_else(
			|| Value::float(number.as_f64().unwrap_or_default(), span),
			|value| Value::int(value, span),
		),
		JsonValue::String(value) => Value::string(value, span),
		JsonValue::Array(values) => Value::list(
			values.into_iter().map(|value| json_to_nu_value(value, span)).collect(),
			span,
		),
		JsonValue::Object(values) => Value::record(
			values
				.into_iter()
				.map(|(key, value)| (key, json_to_nu_value(value, span)))
				.collect::<Record>(),
			span,
		),
	}
}

fn to_labeled_error(error: GlorpError) -> LabeledError {
	match error {
		GlorpError::Validation {
			path,
			message,
			allowed_values,
		} => {
			let mut error = LabeledError::new(message);
			if let Some(path) = path {
				error = error.with_help(format!("path: {path}"));
			}
			if !allowed_values.is_empty() {
				error = error.with_help(format!("allowed values: {}", allowed_values.join(", ")));
			}
			error
		}
		other => LabeledError::new(other.to_string()),
	}
}
