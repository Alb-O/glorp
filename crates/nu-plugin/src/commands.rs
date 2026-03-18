use {
	crate::plugin::GlorpPlugin,
	glorp_api::{
		GlorpError, GlorpEventStreamView, GlorpHelper, GlorpHelperResult, GlorpHost, GlorpOutcome, GlorpQueryResult,
		GlorpSessionView, GlorpValue, OkView, TokenAckView, build_exec, build_helper, build_query,
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

macro_rules! serialize_to_value {
	($value:expr, $span:expr) => {
		json_to_nu_value(serde_json::to_value($value).unwrap_or(JsonValue::Null), $span)
	};
}

pub fn all_commands() -> Vec<Box<dyn PluginCommand<Plugin = GlorpPlugin>>> {
	[
		OperationCommand::new("glorp exec", OperationKind::Exec),
		OperationCommand::new("glorp query", OperationKind::Query),
		OperationCommand::new("glorp helper", OperationKind::Helper),
	]
	.into_iter()
	.map(|command| Box::new(command) as Box<dyn PluginCommand<Plugin = GlorpPlugin>>)
	.collect()
}

#[derive(Debug, Clone, Copy)]
enum OperationKind {
	Exec,
	Query,
	Helper,
}

struct OperationCommand {
	name: &'static str,
	kind: OperationKind,
}

impl OperationCommand {
	const fn new(name: &'static str, kind: OperationKind) -> Self {
		Self { name, kind }
	}
}

impl PluginCommand for OperationCommand {
	type Plugin = GlorpPlugin;

	fn name(&self) -> &str {
		self.name
	}

	fn description(&self) -> &str {
		match self.kind {
			OperationKind::Exec => "Execute a mutating Glorp operation.",
			OperationKind::Query => "Execute a read-only Glorp query.",
			OperationKind::Helper => "Run a plugin-side Glorp helper operation.",
		}
	}

	fn signature(&self) -> Signature {
		Signature::build(self.name)
			.named("socket", SyntaxShape::String, "Runtime Unix socket path.", Some('s'))
			.named("session", SyntaxShape::Any, "Resolved Glorp session record.", None)
			.named(
				"repo-root",
				SyntaxShape::String,
				"Repo root for shared runtime discovery.",
				Some('r'),
			)
			.required("operation", SyntaxShape::String, "Operation identifier.")
			.optional("input", SyntaxShape::Any, "Optional operation input.")
			.input_output_types(vec![(Type::Nothing, Type::Any)])
			.category(Category::Custom("glorp".to_owned()))
	}

	fn run(
		&self, _plugin: &Self::Plugin, _engine: &nu_plugin::EngineInterface, call: &EvaluatedCall, _input: PipelineData,
	) -> Result<PipelineData, LabeledError> {
		let span = call.head;
		let operation: String = call.req(0)?;
		let input = call.opt::<Value>(1)?.map(glorp_value).transpose()?;

		match self.kind {
			OperationKind::Exec => {
				let mut client = resolve_session(call)?.client;
				let outcome = client
					.execute(build_exec(&operation, input.as_ref()).map_err(to_labeled_error)?)
					.map_err(to_labeled_error)?;
				Ok(value_pipeline(outcome_to_value(outcome, span)))
			}
			OperationKind::Query => {
				let mut client = resolve_session(call)?.client;
				let result = client
					.query(build_query(&operation, input.as_ref()).map_err(to_labeled_error)?)
					.map_err(to_labeled_error)?;
				Ok(value_pipeline(query_to_value(result, span)))
			}
			OperationKind::Helper => run_helper(
				build_helper(&operation, input.as_ref()).map_err(to_labeled_error)?,
				call,
				span,
			),
		}
	}
}

fn run_helper(helper: GlorpHelper, call: &EvaluatedCall, span: Span) -> Result<PipelineData, LabeledError> {
	let result = match helper {
		GlorpHelper::ConfigValidate(input) => {
			glorp_api::GlorpConfig::validate_path(&input.path, input.value).map_err(to_labeled_error)?;
			GlorpHelperResult::ConfigValidate(OkView { ok: true })
		}
		GlorpHelper::SessionAttach => {
			let mut resolved = resolve_session(call)?;
			let capabilities = capabilities(&mut resolved.client)?;
			GlorpHelperResult::SessionAttach(GlorpSessionView {
				socket: resolved.socket.display().to_string(),
				repo_root: Some(resolved.repo_root.display().to_string()),
				capabilities,
			})
		}
		GlorpHelper::SessionShutdown => {
			let resolved = resolve_session(call)?;
			let TransportResponse::Shutdown(result) =
				transport_request(&resolved.socket, &TransportRequest::Shutdown).map_err(to_labeled_error)?
			else {
				return Err(LabeledError::new("unexpected shutdown response"));
			};
			result.map_err(to_labeled_error)?;
			GlorpHelperResult::SessionShutdown(OkView { ok: true })
		}
		GlorpHelper::EventsSubscribe => {
			let mut client = resolve_session(call)?.client;
			let token = client
				.subscribe(glorp_api::GlorpSubscription::Changes)
				.map_err(to_labeled_error)?;
			GlorpHelperResult::EventsSubscribe(GlorpEventStreamView {
				token,
				subscription: "changes".to_owned(),
			})
		}
		GlorpHelper::EventsNext(input) => {
			let mut client = resolve_session(call)?.client;
			let event = client.next_event(input.token).map_err(to_labeled_error)?;
			GlorpHelperResult::EventsNext(event)
		}
		GlorpHelper::EventsUnsubscribe(input) => {
			let mut client = resolve_session(call)?.client;
			client.unsubscribe(input.token).map_err(to_labeled_error)?;
			GlorpHelperResult::EventsUnsubscribe(TokenAckView {
				ok: true,
				token: input.token,
			})
		}
	};

	Ok(value_pipeline(helper_to_value(result, span)))
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

	let mut client = IpcClient::new(socket.as_path());
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

	Ok(std::env::current_exe()
		.ok()
		.map(|current| current.with_file_name("glorp_host"))
		.filter(|sibling| sibling.exists())
		.unwrap_or_else(|| PathBuf::from("glorp_host")))
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

fn session_socket(value: &Value) -> Result<PathBuf, LabeledError> {
	let Value::Record { val, .. } = value else {
		return Err(LabeledError::new(
			"session flag must be a record returned by `glorp helper session-attach`",
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
			"session flag must be a record returned by `glorp helper session-attach`",
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

const fn value_pipeline(value: Value) -> PipelineData {
	PipelineData::Value(value, None)
}

fn outcome_to_value(outcome: GlorpOutcome, span: Span) -> Value {
	serialize_to_value!(outcome, span)
}

fn query_to_value(result: GlorpQueryResult, span: Span) -> Value {
	match result {
		GlorpQueryResult::Schema(value) => serialize_to_value!(value, span),
		GlorpQueryResult::Config(value) => serialize_to_value!(value, span),
		GlorpQueryResult::DocumentText(value) => serialize_to_value!(value, span),
		GlorpQueryResult::Editor(value) => serialize_to_value!(value, span),
		GlorpQueryResult::Capabilities(value) => serialize_to_value!(value, span),
	}
}

fn helper_to_value(result: GlorpHelperResult, span: Span) -> Value {
	match result {
		GlorpHelperResult::SessionAttach(value) => serialize_to_value!(value, span),
		GlorpHelperResult::SessionShutdown(value) => serialize_to_value!(value, span),
		GlorpHelperResult::ConfigValidate(value) => serialize_to_value!(value, span),
		GlorpHelperResult::EventsSubscribe(value) => serialize_to_value!(value, span),
		GlorpHelperResult::EventsNext(value) => serialize_to_value!(value, span),
		GlorpHelperResult::EventsUnsubscribe(value) => serialize_to_value!(value, span),
	}
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
