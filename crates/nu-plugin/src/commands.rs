use {
	crate::plugin::GlorpPlugin,
	glorp_api::*,
	glorp_transport::IpcClient,
	nu_plugin::{EvaluatedCall, PluginCommand},
	nu_protocol::{Category, LabeledError, PipelineData, Record, Signature, Span, SyntaxShape, Type, Value},
	serde_json::Value as JsonValue,
};

pub fn all_commands() -> Vec<Box<dyn PluginCommand<Plugin = GlorpPlugin>>> {
	vec![
		Box::new(GlorpSchema),
		Box::new(GlorpGetConfig),
		Box::new(GlorpGetState),
		Box::new(GlorpGetDocumentText),
		Box::new(GlorpGetSelection),
		Box::new(GlorpGetInspectDetails),
		Box::new(GlorpGetPerf),
		Box::new(GlorpGetUi),
		Box::new(GlorpSessionAttach),
		Box::new(GlorpConfigSet),
		Box::new(GlorpConfigPatch),
		Box::new(GlorpConfigReset),
		Box::new(GlorpConfigValidate),
		Box::new(GlorpDocReplace),
		Box::new(GlorpEditorMotion),
		Box::new(GlorpEditorMode),
		Box::new(GlorpEditorEditInsert),
		Box::new(GlorpEditorHistory),
		Box::new(GlorpUiSidebarSelect),
		Box::new(GlorpUiViewportScrollTo),
		Box::new(GlorpSceneEnsure),
		Box::new(GlorpEventsSubscribe),
		Box::new(GlorpEventsNext),
		Box::new(GlorpEventsUnsubscribe),
	]
}

struct GlorpSchema;
struct GlorpGetConfig;
struct GlorpGetState;
struct GlorpGetDocumentText;
struct GlorpGetSelection;
struct GlorpGetInspectDetails;
struct GlorpGetPerf;
struct GlorpGetUi;
struct GlorpSessionAttach;
struct GlorpConfigSet;
struct GlorpConfigPatch;
struct GlorpConfigReset;
struct GlorpConfigValidate;
struct GlorpDocReplace;
struct GlorpEditorMotion;
struct GlorpEditorMode;
struct GlorpEditorEditInsert;
struct GlorpEditorHistory;
struct GlorpUiSidebarSelect;
struct GlorpUiViewportScrollTo;
struct GlorpSceneEnsure;
struct GlorpEventsSubscribe;
struct GlorpEventsNext;
struct GlorpEventsUnsubscribe;

macro_rules! impl_simple_command {
	($name:ident, $cmd_name:literal, $desc:literal, $sig:expr, $run:expr) => {
		impl PluginCommand for $name {
			type Plugin = GlorpPlugin;

			fn name(&self) -> &str {
				$cmd_name
			}

			fn description(&self) -> &str {
				$desc
			}

			fn signature(&self) -> Signature {
				$sig
			}

			fn run(
				&self, _plugin: &Self::Plugin, _engine: &nu_plugin::EngineInterface, call: &EvaluatedCall,
				input: PipelineData,
			) -> Result<PipelineData, LabeledError> {
				let run: fn(&EvaluatedCall, Span, PipelineData) -> Result<PipelineData, LabeledError> = $run;
				run(call, call.head, input)
			}
		}
	};
}

impl_simple_command!(
	GlorpSchema,
	"glorp schema",
	"Return the Glorp reflection schema.",
	base_signature("glorp schema"),
	|call, span, _input| {
		let mut client = client_from_call(call)?;
		let result = client.query(GlorpQuery::Schema).map_err(to_labeled_error)?;
		Ok(value_pipeline(query_to_value(result, span)))
	}
);

impl_simple_command!(
	GlorpGetConfig,
	"glorp get config",
	"Return the effective runtime config.",
	base_signature("glorp get config"),
	|call, span, _input| {
		let mut client = client_from_call(call)?;
		let result = client.query(GlorpQuery::Config).map_err(to_labeled_error)?;
		Ok(value_pipeline(query_to_value(result, span)))
	}
);

impl_simple_command!(
	GlorpGetState,
	"glorp get state",
	"Return a runtime snapshot.",
	base_signature("glorp get state"),
	|call, span, _input| {
		let mut client = client_from_call(call)?;
		let result = client
			.query(GlorpQuery::Snapshot {
				scene: SceneLevel::Materialize,
				include_document_text: true,
			})
			.map_err(to_labeled_error)?;
		Ok(value_pipeline(query_to_value(result, span)))
	}
);

impl_simple_command!(
	GlorpGetDocumentText,
	"glorp get document-text",
	"Return document text.",
	base_signature("glorp get document-text"),
	|call, span, _input| {
		let mut client = client_from_call(call)?;
		let result = client.query(GlorpQuery::DocumentText).map_err(to_labeled_error)?;
		Ok(value_pipeline(query_to_value(result, span)))
	}
);

impl_simple_command!(
	GlorpGetSelection,
	"glorp get selection",
	"Return the current selection read model.",
	base_signature("glorp get selection"),
	|call, span, _input| {
		let mut client = client_from_call(call)?;
		let result = client.query(GlorpQuery::Selection).map_err(to_labeled_error)?;
		Ok(value_pipeline(query_to_value(result, span)))
	}
);

impl_simple_command!(
	GlorpGetInspectDetails,
	"glorp get inspect-details",
	"Return the current inspect read model.",
	base_signature("glorp get inspect-details").optional(
		"target",
		SyntaxShape::String,
		"Optional canvas target run:<n> or cluster:<n>."
	),
	|call, span, _input| {
		let mut client = client_from_call(call)?;
		let target: Option<String> = call.opt(0)?;
		let result = client
			.query(GlorpQuery::InspectDetails {
				target: target.as_deref().map(parse_canvas_target).transpose()?,
			})
			.map_err(to_labeled_error)?;
		Ok(value_pipeline(query_to_value(result, span)))
	}
);

impl_simple_command!(
	GlorpGetPerf,
	"glorp get perf",
	"Return the runtime perf dashboard.",
	base_signature("glorp get perf"),
	|call, span, _input| {
		let mut client = client_from_call(call)?;
		let result = client.query(GlorpQuery::PerfDashboard).map_err(to_labeled_error)?;
		Ok(value_pipeline(query_to_value(result, span)))
	}
);

impl_simple_command!(
	GlorpGetUi,
	"glorp get ui",
	"Return the current UI state.",
	base_signature("glorp get ui"),
	|call, span, _input| {
		let mut client = client_from_call(call)?;
		let result = client.query(GlorpQuery::UiState).map_err(to_labeled_error)?;
		Ok(value_pipeline(query_to_value(result, span)))
	}
);

impl_simple_command!(
	GlorpSessionAttach,
	"glorp session attach",
	"Resolve and validate a live Glorp session.",
	base_signature("glorp session attach"),
	|call, span, _input| {
		let socket = socket_from_call(call)?;
		let mut client = IpcClient::new(socket.clone());
		let capabilities = match client.query(GlorpQuery::Capabilities).map_err(to_labeled_error)? {
			GlorpQueryResult::Capabilities(capabilities) => capabilities,
			_ => return Err(LabeledError::new("unexpected capabilities response")),
		};
		Ok(value_pipeline(json_to_nu_value(
			serde_json::to_value(GlorpSessionView {
				socket,
				repo_root: None,
				capabilities,
			})
			.unwrap_or(JsonValue::Null),
			span,
		)))
	}
);

impl_simple_command!(
	GlorpConfigSet,
	"glorp config set",
	"Set one config field.",
	base_signature("glorp config set")
		.required("path", SyntaxShape::String, "Config path.")
		.required("value", SyntaxShape::Any, "Config value."),
	|call, span, _input| {
		let mut client = client_from_call(call)?;
		let path: String = call.req(0)?;
		let value: Value = call.req(1)?;
		let outcome = client
			.execute(GlorpCommand::Config(ConfigCommand::Set {
				path,
				value: glorp_value(value)?,
			}))
			.map_err(to_labeled_error)?;
		Ok(value_pipeline(outcome_to_value(outcome, span)))
	}
);

impl_simple_command!(
	GlorpConfigPatch,
	"glorp config patch",
	"Patch config with a record.",
	base_signature("glorp config patch").required("patch", SyntaxShape::Any, "Nested config patch record."),
	|call, span, _input| {
		let mut client = client_from_call(call)?;
		let patch: Value = call.req(0)?;
		let assignments = flatten_record(None, glorp_value(patch)?)?;
		let outcome = client
			.execute(GlorpCommand::Config(ConfigCommand::Patch { values: assignments }))
			.map_err(to_labeled_error)?;
		Ok(value_pipeline(outcome_to_value(outcome, span)))
	}
);

impl_simple_command!(
	GlorpConfigReset,
	"glorp config reset",
	"Reset one config field to its default.",
	base_signature("glorp config reset").required("path", SyntaxShape::String, "Config path."),
	|call, span, _input| {
		let mut client = client_from_call(call)?;
		let path: String = call.req(0)?;
		let outcome = client
			.execute(GlorpCommand::Config(ConfigCommand::Reset { path }))
			.map_err(to_labeled_error)?;
		Ok(value_pipeline(outcome_to_value(outcome, span)))
	}
);

impl_simple_command!(
	GlorpConfigValidate,
	"glorp config validate",
	"Validate a config value without mutating runtime state.",
	base_signature("glorp config validate")
		.required("path", SyntaxShape::String, "Config path.")
		.required("value", SyntaxShape::Any, "Candidate value."),
	|call, span, _input| {
		let path: String = call.req(0)?;
		let value: Value = call.req(1)?;
		GlorpConfig::validate_path(&path, glorp_value(value)?).map_err(to_labeled_error)?;
		Ok(value_pipeline(Value::record(
			Record::from_iter([("ok".to_owned(), Value::bool(true, span))]),
			span,
		)))
	}
);

impl_simple_command!(
	GlorpDocReplace,
	"glorp doc replace",
	"Replace the document text.",
	base_signature("glorp doc replace").required("text", SyntaxShape::String, "Document text."),
	|call, span, _input| {
		let mut client = client_from_call(call)?;
		let text: String = call.req(0)?;
		let outcome = client
			.execute(GlorpCommand::Document(DocumentCommand::Replace { text }))
			.map_err(to_labeled_error)?;
		Ok(value_pipeline(outcome_to_value(outcome, span)))
	}
);

impl_simple_command!(
	GlorpEditorMotion,
	"glorp editor motion",
	"Apply a typed motion command.",
	base_signature("glorp editor motion").required("motion", SyntaxShape::String, "Motion name."),
	|call, span, _input| {
		let mut client = client_from_call(call)?;
		let motion: String = call.req(0)?;
		let motion = parse_motion(&motion)?;
		let outcome = client
			.execute(GlorpCommand::Editor(EditorCommand::Motion(motion)))
			.map_err(to_labeled_error)?;
		Ok(value_pipeline(outcome_to_value(outcome, span)))
	}
);

impl_simple_command!(
	GlorpEditorMode,
	"glorp editor mode",
	"Apply a typed mode command.",
	base_signature("glorp editor mode").required("mode", SyntaxShape::String, "Mode command."),
	|call, span, _input| {
		let mut client = client_from_call(call)?;
		let mode: String = call.req(0)?;
		let mode = parse_mode(&mode)?;
		let outcome = client
			.execute(GlorpCommand::Editor(EditorCommand::Mode(mode)))
			.map_err(to_labeled_error)?;
		Ok(value_pipeline(outcome_to_value(outcome, span)))
	}
);

impl_simple_command!(
	GlorpEditorEditInsert,
	"glorp editor edit insert",
	"Insert text.",
	base_signature("glorp editor edit insert").required("text", SyntaxShape::String, "Text to insert."),
	|call, span, _input| {
		let mut client = client_from_call(call)?;
		let text: String = call.req(0)?;
		let outcome = client
			.execute(GlorpCommand::Editor(EditorCommand::Edit(EditorEditCommand::Insert {
				text,
			})))
			.map_err(to_labeled_error)?;
		Ok(value_pipeline(outcome_to_value(outcome, span)))
	}
);

impl_simple_command!(
	GlorpEditorHistory,
	"glorp editor history",
	"Apply history navigation.",
	base_signature("glorp editor history").required("action", SyntaxShape::String, "undo or redo."),
	|call, span, _input| {
		let mut client = client_from_call(call)?;
		let action: String = call.req(0)?;
		let history = parse_history(&action)?;
		let outcome = client
			.execute(GlorpCommand::Editor(EditorCommand::History(history)))
			.map_err(to_labeled_error)?;
		Ok(value_pipeline(outcome_to_value(outcome, span)))
	}
);

impl_simple_command!(
	GlorpUiSidebarSelect,
	"glorp ui sidebar select",
	"Select a sidebar tab.",
	base_signature("glorp ui sidebar select").required("tab", SyntaxShape::String, "Sidebar tab."),
	|call, span, _input| {
		let mut client = client_from_call(call)?;
		let tab: String = call.req(0)?;
		let outcome = client
			.execute(GlorpCommand::Ui(UiCommand::SidebarSelect { tab: parse_tab(&tab)? }))
			.map_err(to_labeled_error)?;
		Ok(value_pipeline(outcome_to_value(outcome, span)))
	}
);

impl_simple_command!(
	GlorpUiViewportScrollTo,
	"glorp ui viewport scroll-to",
	"Set viewport scroll position.",
	base_signature("glorp ui viewport scroll-to")
		.required("x", SyntaxShape::Number, "X scroll.")
		.required("y", SyntaxShape::Number, "Y scroll."),
	|call, span, _input| {
		let mut client = client_from_call(call)?;
		let x: f64 = call.req(0)?;
		let y: f64 = call.req(1)?;
		let outcome = client
			.execute(GlorpCommand::Ui(UiCommand::ViewportScrollTo {
				x: x as f32,
				y: y as f32,
			}))
			.map_err(to_labeled_error)?;
		Ok(value_pipeline(outcome_to_value(outcome, span)))
	}
);

impl_simple_command!(
	GlorpSceneEnsure,
	"glorp scene ensure",
	"Materialize scene state.",
	base_signature("glorp scene ensure"),
	|call, span, _input| {
		let mut client = client_from_call(call)?;
		let outcome = client
			.execute(GlorpCommand::Scene(SceneCommand::Ensure))
			.map_err(to_labeled_error)?;
		Ok(value_pipeline(outcome_to_value(outcome, span)))
	}
);

impl_simple_command!(
	GlorpEventsSubscribe,
	"glorp events subscribe",
	"Subscribe to runtime change events.",
	base_signature("glorp events subscribe"),
	|call, span, _input| {
		let mut client = client_from_call(call)?;
		let token = client.subscribe(GlorpSubscription::Changes).map_err(to_labeled_error)?;
		Ok(value_pipeline(json_to_nu_value(
			serde_json::to_value(GlorpEventStreamView {
				token,
				subscription: "changes".to_owned(),
			})
			.unwrap_or(JsonValue::Null),
			span,
		)))
	}
);

impl_simple_command!(
	GlorpEventsNext,
	"glorp events next",
	"Poll the next event for a subscription token.",
	base_signature("glorp events next").required("token", SyntaxShape::Int, "Subscription token."),
	|call, span, _input| {
		let mut client = client_from_call(call)?;
		let token: i64 = call.req(0)?;
		let event = client.next_event(token as u64).map_err(to_labeled_error)?;
		Ok(value_pipeline(json_to_nu_value(
			serde_json::to_value(event).unwrap_or(JsonValue::Null),
			span,
		)))
	}
);

impl_simple_command!(
	GlorpEventsUnsubscribe,
	"glorp events unsubscribe",
	"Release a subscription token.",
	base_signature("glorp events unsubscribe").required("token", SyntaxShape::Int, "Subscription token."),
	|call, span, _input| {
		let mut client = client_from_call(call)?;
		let token: i64 = call.req(0)?;
		client.unsubscribe(token as u64).map_err(to_labeled_error)?;
		Ok(value_pipeline(json_to_nu_value(
			serde_json::json!({"ok": true, "token": token}),
			span,
		)))
	}
);

fn base_signature(name: &str) -> Signature {
	Signature::build(name)
		.named("socket", SyntaxShape::String, "Runtime Unix socket path.", Some('s'))
		.named(
			"session",
			SyntaxShape::Any,
			"Session record returned by glorp session attach.",
			None,
		)
		.input_output_types(vec![(Type::Nothing, Type::Any)])
		.category(Category::Custom("glorp".to_owned()))
}

fn client_from_call(call: &EvaluatedCall) -> Result<IpcClient, LabeledError> {
	Ok(IpcClient::new(socket_from_call(call)?))
}

fn socket_from_call(call: &EvaluatedCall) -> Result<String, LabeledError> {
	let session: Option<Value> = call.get_flag("session")?;
	let socket: Option<String> = call.get_flag("socket")?;
	session
		.as_ref()
		.map(session_socket)
		.transpose()?
		.or(socket)
		.or_else(|| std::env::var("GLORP_SOCKET").ok())
		.ok_or_else(|| LabeledError::new("GLORP_SOCKET is not set and neither --socket nor --session was provided"))
}

fn value_pipeline(value: Value) -> PipelineData {
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
		Value::Record { val, .. } => {
			let mut record = std::collections::BTreeMap::new();
			for (key, value) in val.into_owned() {
				record.insert(key, glorp_value(value)?);
			}
			Ok(GlorpValue::Record(record))
		}
		other => Err(LabeledError::new(format!(
			"unsupported value type `{}`",
			other.get_type()
		))),
	}
}

fn flatten_record(prefix: Option<&str>, value: GlorpValue) -> Result<Vec<ConfigAssignment>, LabeledError> {
	match value {
		GlorpValue::Record(fields) => {
			let mut assignments = Vec::new();
			for (key, value) in fields {
				let path = prefix.map(|prefix| format!("{prefix}.{key}")).unwrap_or(key);
				assignments.extend(flatten_record(Some(&path), value)?);
			}
			Ok(assignments)
		}
		value => Ok(vec![ConfigAssignment {
			path: prefix.unwrap_or_default().to_owned(),
			value,
		}]),
	}
}

fn parse_motion(value: &str) -> Result<EditorMotion, LabeledError> {
	match value {
		"left" => Ok(EditorMotion::Left),
		"right" => Ok(EditorMotion::Right),
		"up" => Ok(EditorMotion::Up),
		"down" => Ok(EditorMotion::Down),
		"line-start" => Ok(EditorMotion::LineStart),
		"line-end" => Ok(EditorMotion::LineEnd),
		_ => Err(LabeledError::new(format!("unknown motion `{value}`"))),
	}
}

fn parse_mode(value: &str) -> Result<EditorModeCommand, LabeledError> {
	match value {
		"enter-insert-before" => Ok(EditorModeCommand::EnterInsertBefore),
		"enter-insert-after" => Ok(EditorModeCommand::EnterInsertAfter),
		"exit-insert" => Ok(EditorModeCommand::ExitInsert),
		_ => Err(LabeledError::new(format!("unknown mode `{value}`"))),
	}
}

fn parse_history(value: &str) -> Result<EditorHistoryCommand, LabeledError> {
	match value {
		"undo" => Ok(EditorHistoryCommand::Undo),
		"redo" => Ok(EditorHistoryCommand::Redo),
		_ => Err(LabeledError::new(format!("unknown history action `{value}`"))),
	}
}

fn parse_tab(value: &str) -> Result<SidebarTab, LabeledError> {
	match value {
		"controls" => Ok(SidebarTab::Controls),
		"inspect" => Ok(SidebarTab::Inspect),
		"perf" => Ok(SidebarTab::Perf),
		_ => Err(LabeledError::new(format!("unknown tab `{value}`"))),
	}
}

fn parse_canvas_target(value: &str) -> Result<CanvasTarget, LabeledError> {
	let (kind, index) = value
		.split_once(':')
		.ok_or_else(|| LabeledError::new(format!("invalid canvas target `{value}`")))?;
	let index = index
		.parse::<usize>()
		.map_err(|error| LabeledError::new(format!("invalid canvas target `{value}`: {error}")))?;
	match kind {
		"run" => Ok(CanvasTarget::Run(index)),
		"cluster" => Ok(CanvasTarget::Cluster(index)),
		_ => Err(LabeledError::new(format!("unknown canvas target kind `{kind}`"))),
	}
}

fn session_socket(value: &Value) -> Result<String, LabeledError> {
	match value {
		Value::Record { val, .. } => val
			.get("socket")
			.and_then(|value| match value {
				Value::String { val, .. } => Some(val.clone()),
				_ => None,
			})
			.ok_or_else(|| LabeledError::new("session record does not contain a string `socket` field")),
		_ => Err(LabeledError::new(
			"session flag must be a record returned by glorp session attach",
		)),
	}
}

fn json_to_nu_value(value: JsonValue, span: Span) -> Value {
	match value {
		JsonValue::Null => Value::nothing(span),
		JsonValue::Bool(value) => Value::bool(value, span),
		JsonValue::Number(number) => {
			if let Some(value) = number.as_i64() {
				Value::int(value, span)
			} else {
				Value::float(number.as_f64().unwrap_or_default(), span)
			}
		}
		JsonValue::String(value) => Value::string(value, span),
		JsonValue::Array(values) => Value::list(
			values.into_iter().map(|value| json_to_nu_value(value, span)).collect(),
			span,
		),
		JsonValue::Object(values) => {
			let mut record = Record::new();
			for (key, value) in values {
				record.push(key, json_to_nu_value(value, span));
			}
			Value::record(record, span)
		}
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
