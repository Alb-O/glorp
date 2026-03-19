use {
	crate::plugin::GlorpPlugin,
	glorp_api::{
		ClientCallDispatcher, GlorpCallDescriptor, GlorpCallResult, GlorpCallRoute, GlorpCaller, GlorpError,
		GlorpValue, OkView, build_call, call_spec, dispatch_client_call,
	},
	glorp_transport::{IpcClient, default_socket_path, socket_is_live},
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
	vec![Box::new(OperationCommand::new())]
}

struct OperationCommand {
	name: &'static str,
}

impl OperationCommand {
	const fn new() -> Self {
		Self { name: "glorp call" }
	}
}

impl PluginCommand for OperationCommand {
	type Plugin = GlorpPlugin;

	fn name(&self) -> &str {
		self.name
	}

	fn description(&self) -> &str {
		"Execute a public Glorp call."
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
			.required("operation", SyntaxShape::String, "Call identifier.")
			.optional("input", SyntaxShape::Any, "Optional call input.")
			.input_output_types(vec![(Type::Nothing, Type::Any)])
			.category(Category::Custom("glorp".to_owned()))
	}

	fn run(
		&self, _plugin: &Self::Plugin, _engine: &nu_plugin::EngineInterface, call: &EvaluatedCall, _input: PipelineData,
	) -> Result<PipelineData, LabeledError> {
		let span = call.head;
		let operation: String = call.req(0)?;
		let input = call.opt::<Value>(1)?.map(glorp_value).transpose()?;
		let glorp_call = build_call(&operation, input.as_ref()).map_err(to_labeled_error)?;
		let Some(spec) = call_spec(&glorp_call.id) else {
			return Err(LabeledError::new(format!("unknown call `{}`", glorp_call.id)));
		};

		let result = match spec.route {
			GlorpCallRoute::Client => {
				dispatch_client_call(&mut PluginClientDispatcher { call }, glorp_call).map_err(to_labeled_error)?
			}
			GlorpCallRoute::Runtime | GlorpCallRoute::Transport => {
				let mut client = resolve_session(call)?.client;
				client.call(glorp_call).map_err(to_labeled_error)?
			}
		};

		Ok(value_pipeline(call_to_value(result, span)))
	}
}

struct ResolvedSession {
	repo_root: PathBuf,
	socket: PathBuf,
	client: IpcClient,
}

fn resolve_session(call: &EvaluatedCall) -> Result<ResolvedSession, LabeledError> {
	if let Some(session) = call.get_flag::<Value>("session")? {
		let socket = PathBuf::from(session_socket(&session)?);
		let repo_root = session_repo_root(&session)?.map_or_else(repo_root_from_call, PathBuf::from);
		return ensure_session(repo_root, socket);
	}

	if let Some(socket) = call.get_flag::<String>("socket")? {
		return ensure_session(repo_root_from_call(), PathBuf::from(socket));
	}

	let repo_root = call
		.get_flag::<String>("repo-root")?
		.map_or_else(repo_root_from_call, PathBuf::from);
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

fn capabilities(client: &mut impl GlorpCaller) -> Result<glorp_api::GlorpCapabilities, LabeledError> {
	glorp_api::calls::Capabilities::call(client, ()).map_err(to_labeled_error)
}

fn spawn_host(repo_root: &Path, socket: &Path) -> Result<(), LabeledError> {
	Command::new(host_binary_path())
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

fn host_binary_path() -> PathBuf {
	std::env::var_os("GLORP_HOST_BIN")
		.map(PathBuf::from)
		.or_else(|| {
			std::env::current_exe()
				.ok()
				.map(|current| current.with_file_name("glorp_host"))
				.filter(|sibling| sibling.exists())
		})
		.unwrap_or_else(|| PathBuf::from("glorp_host"))
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

fn session_socket(value: &Value) -> Result<&str, LabeledError> {
	let Value::Record { val, .. } = value else {
		return Err(LabeledError::new(
			"session flag must be a record returned by `glorp call session-attach`",
		));
	};

	match val.get("socket") {
		Some(Value::String { val, .. }) => Ok(val),
		_ => Err(LabeledError::new(
			"session record does not contain a string `socket` field",
		)),
	}
}

fn session_repo_root(value: &Value) -> Result<Option<&str>, LabeledError> {
	let Value::Record { val, .. } = value else {
		return Err(LabeledError::new(
			"session flag must be a record returned by `glorp call session-attach`",
		));
	};

	match val.get("repo_root") {
		Some(Value::String { val, .. }) => Ok(Some(val)),
		Some(Value::Nothing { .. }) | None => Ok(None),
		_ => Err(LabeledError::new(
			"session record contains a non-string `repo_root` field",
		)),
	}
}

const fn value_pipeline(value: Value) -> PipelineData {
	PipelineData::Value(value, None)
}

fn call_to_value(result: GlorpCallResult, span: Span) -> Value {
	json_to_nu_value((&result.output).into(), span)
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

struct PluginClientDispatcher<'a> {
	call: &'a EvaluatedCall,
}

impl ClientCallDispatcher for PluginClientDispatcher<'_> {
	fn session_attach(&mut self, _input: ()) -> Result<glorp_api::GlorpSessionView, GlorpError> {
		let mut resolved = resolve_session(self.call).map_err(|error| GlorpError::transport(error.to_string()))?;
		let capabilities = glorp_api::calls::Capabilities::call(&mut resolved.client, ())?;
		Ok(glorp_api::GlorpSessionView {
			socket: resolved.socket.display().to_string(),
			repo_root: Some(resolved.repo_root.display().to_string()),
			capabilities,
		})
	}

	fn config_validate(&mut self, input: glorp_api::ConfigAssignment) -> Result<OkView, GlorpError> {
		glorp_api::GlorpConfig::validate_path(&input.path, &input.value)?;
		Ok(OkView { ok: true })
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
			Record::from_iter(
				values
					.into_iter()
					.map(|(key, value)| (key, json_to_nu_value(value, span))),
			),
			span,
		),
	}
}

fn to_labeled_error(error: GlorpError) -> LabeledError {
	LabeledError::new(error.to_string())
}
