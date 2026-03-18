use {
	crate::{
		BuiltinType, CanvasTarget, CommandSchema, ConfigAssignment, ConfigCommand, DocumentCommand, EditorCommand,
		EditorEditCommand, GlorpCommand, GlorpError, GlorpInvocation, GlorpQuery, GlorpTxn, GlorpValue, HelperKind,
		HelperSchema, QuerySchema, SceneCommand, TypeRef, UiCommand,
	},
	std::collections::BTreeMap,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceKind {
	Command,
	Query,
	Helper(HelperKind),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceArgKind {
	String,
	Int,
	Float,
	Bool,
	Any,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceArg {
	pub name: &'static str,
	pub docs: &'static str,
	pub kind: SurfaceArgKind,
	pub required: bool,
	pub completion: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceSpec {
	pub path: &'static str,
	pub docs: &'static str,
	pub kind: SurfaceKind,
	pub args: Vec<SurfaceArg>,
	pub input: Option<TypeRef>,
	pub output: TypeRef,
}

pub fn surface_specs() -> Vec<SurfaceSpec> {
	vec![
		query(
			"glorp schema",
			"Return the runtime reflection schema.",
			None,
			vec![],
			named("GlorpSchema"),
		),
		query(
			"glorp get config",
			"Return the effective runtime config.",
			None,
			vec![],
			named("GlorpConfig"),
		),
		query(
			"glorp get state",
			"Return a snapshot of runtime state.",
			None,
			vec![],
			named("GlorpSnapshot"),
		),
		query(
			"glorp get document-text",
			"Return the current document text.",
			None,
			vec![],
			built(BuiltinType::String),
		),
		query(
			"glorp get selection",
			"Return the current selection read model.",
			None,
			vec![],
			named("SelectionStateView"),
		),
		query(
			"glorp get inspect-details",
			"Return the current inspect read model.",
			Some(named("InspectDetailsInput")),
			vec![arg(
				"target",
				"Optional canvas target run:<n> or cluster:<n>.",
				SurfaceArgKind::String,
				false,
				None,
			)],
			named("InspectDetailsView"),
		),
		query(
			"glorp get perf",
			"Return the runtime perf dashboard.",
			None,
			vec![],
			named("PerfDashboardView"),
		),
		query(
			"glorp get ui",
			"Return the current UI state.",
			None,
			vec![],
			named("UiStateView"),
		),
		query(
			"glorp get capabilities",
			"Return stable runtime capability flags.",
			None,
			vec![],
			named("GlorpCapabilities"),
		),
		helper(
			"glorp session attach",
			"Resolve and validate a live Glorp session.",
			HelperKind::SessionAttach,
			None,
			vec![],
			named("GlorpSessionView"),
		),
		helper(
			"glorp session shutdown",
			"Stop the live shared runtime for the resolved session.",
			HelperKind::SessionShutdown,
			None,
			vec![],
			named("OkView"),
		),
		command(
			"glorp config set",
			"Set one config field.",
			Some(named("ConfigAssignment")),
			vec![
				arg("path", "Config path.", SurfaceArgKind::String, true, None),
				arg("value", "Config value.", SurfaceArgKind::Any, true, None),
			],
			named("GlorpOutcome"),
		),
		command(
			"glorp config reset",
			"Reset one config field to its default.",
			Some(named("ConfigPathInput")),
			vec![arg("path", "Config path.", SurfaceArgKind::String, true, None)],
			named("GlorpOutcome"),
		),
		command(
			"glorp config patch",
			"Patch config with a record.",
			Some(named("ConfigPatch")),
			vec![arg(
				"patch",
				"Nested config patch record.",
				SurfaceArgKind::Any,
				true,
				None,
			)],
			named("GlorpOutcome"),
		),
		helper(
			"glorp config validate",
			"Validate a config value without mutating runtime state.",
			HelperKind::ConfigValidate,
			Some(named("ConfigAssignment")),
			vec![
				arg("path", "Config path.", SurfaceArgKind::String, true, None),
				arg("value", "Candidate value.", SurfaceArgKind::Any, true, None),
			],
			named("OkView"),
		),
		command(
			"glorp config reload",
			"Reload the durable config file.",
			None,
			vec![],
			named("GlorpOutcome"),
		),
		command(
			"glorp config persist",
			"Persist the effective config to the durable config file.",
			None,
			vec![],
			named("GlorpOutcome"),
		),
		command(
			"glorp doc replace",
			"Replace the document text.",
			Some(named("TextInput")),
			vec![arg("text", "Document text.", SurfaceArgKind::String, true, None)],
			named("GlorpOutcome"),
		),
		command(
			"glorp editor motion",
			"Apply a typed motion command.",
			Some(named("EditorMotionInput")),
			vec![arg(
				"motion",
				"Motion name.",
				SurfaceArgKind::String,
				true,
				Some("motion"),
			)],
			named("GlorpOutcome"),
		),
		command(
			"glorp editor mode",
			"Apply a typed mode command.",
			Some(named("EditorModeInput")),
			vec![arg("mode", "Mode command.", SurfaceArgKind::String, true, Some("mode"))],
			named("GlorpOutcome"),
		),
		command(
			"glorp editor edit insert",
			"Insert text.",
			Some(named("TextInput")),
			vec![arg("text", "Text to insert.", SurfaceArgKind::String, true, None)],
			named("GlorpOutcome"),
		),
		command(
			"glorp editor edit backspace",
			"Delete backward.",
			None,
			vec![],
			named("GlorpOutcome"),
		),
		command(
			"glorp editor edit delete-forward",
			"Delete forward.",
			None,
			vec![],
			named("GlorpOutcome"),
		),
		command(
			"glorp editor edit delete-selection",
			"Delete the current selection.",
			None,
			vec![],
			named("GlorpOutcome"),
		),
		command(
			"glorp editor history",
			"Apply history navigation.",
			Some(named("EditorHistoryInput")),
			vec![arg(
				"action",
				"History action.",
				SurfaceArgKind::String,
				true,
				Some("history"),
			)],
			named("GlorpOutcome"),
		),
		command(
			"glorp ui sidebar select",
			"Select a sidebar tab.",
			Some(named("SidebarTabInput")),
			vec![arg("tab", "Sidebar tab.", SurfaceArgKind::String, true, Some("tab"))],
			named("GlorpOutcome"),
		),
		command(
			"glorp ui viewport scroll-to",
			"Set runtime viewport scroll position.",
			Some(named("ScrollTarget")),
			vec![
				arg("x", "X scroll.", SurfaceArgKind::Float, true, None),
				arg("y", "Y scroll.", SurfaceArgKind::Float, true, None),
			],
			named("GlorpOutcome"),
		),
		command(
			"glorp ui pane-ratio-set",
			"Set the sidebar/canvas split ratio.",
			Some(named("PaneRatioInput")),
			vec![arg("ratio", "Pane ratio.", SurfaceArgKind::Float, true, None)],
			named("GlorpOutcome"),
		),
		command(
			"glorp scene ensure",
			"Materialize scene state.",
			None,
			vec![],
			named("GlorpOutcome"),
		),
		helper(
			"glorp events subscribe",
			"Subscribe to runtime change events.",
			HelperKind::EventsSubscribe,
			None,
			vec![],
			named("GlorpEventStreamView"),
		),
		helper(
			"glorp events next",
			"Poll the next event for a subscription token.",
			HelperKind::EventsNext,
			Some(named("StreamTokenInput")),
			vec![arg("token", "Subscription token.", SurfaceArgKind::Int, true, None)],
			built(BuiltinType::Any),
		),
		helper(
			"glorp events unsubscribe",
			"Release a subscription token.",
			HelperKind::EventsUnsubscribe,
			Some(named("StreamTokenInput")),
			vec![arg("token", "Subscription token.", SurfaceArgKind::Int, true, None)],
			named("TokenAckView"),
		),
		command(
			"glorp txn",
			"Apply a transaction atomically.",
			Some(named("GlorpTxn")),
			vec![arg(
				"commands",
				"Ordered command invocations.",
				SurfaceArgKind::Any,
				true,
				None,
			)],
			named("GlorpOutcome"),
		),
	]
}

pub fn command_schemas() -> Vec<CommandSchema> {
	surface_specs()
		.into_iter()
		.filter_map(|spec| match spec.kind {
			SurfaceKind::Command => Some(CommandSchema {
				path: spec.path.to_owned(),
				docs: spec.docs.to_owned(),
				input: spec.input.unwrap_or_else(|| built(BuiltinType::Null)),
				output: spec.output,
			}),
			_ => None,
		})
		.collect()
}

pub fn query_schemas() -> Vec<QuerySchema> {
	surface_specs()
		.into_iter()
		.filter_map(|spec| match spec.kind {
			SurfaceKind::Query => Some(QuerySchema {
				path: spec.path.to_owned(),
				docs: spec.docs.to_owned(),
				input: spec.input,
				output: spec.output,
			}),
			_ => None,
		})
		.collect()
}

pub fn helper_schemas() -> Vec<HelperSchema> {
	surface_specs()
		.into_iter()
		.filter_map(|spec| match spec.kind {
			SurfaceKind::Helper(kind) => Some(HelperSchema {
				path: spec.path.to_owned(),
				docs: spec.docs.to_owned(),
				kind,
				input: spec.input,
				output: spec.output,
			}),
			_ => None,
		})
		.collect()
}

pub fn command_invocation(path: &str, input: Option<&GlorpValue>) -> Result<GlorpCommand, GlorpError> {
	match path {
		"glorp config set" => decode_config_set(input),
		"glorp config reset" => decode_config_reset(input),
		"glorp config patch" => decode_config_patch(input),
		"glorp config reload" => ensure_no_input(path, input).map(|()| GlorpCommand::Config(ConfigCommand::Reload)),
		"glorp config persist" => ensure_no_input(path, input).map(|()| GlorpCommand::Config(ConfigCommand::Persist)),
		"glorp doc replace" => decode_document_replace(input),
		"glorp editor motion" => decode_editor_motion(input),
		"glorp editor mode" => decode_editor_mode(input),
		"glorp editor edit insert" => decode_editor_edit_insert(input),
		"glorp editor edit backspace" => ensure_no_input(path, input)
			.map(|()| GlorpCommand::Editor(EditorCommand::Edit(EditorEditCommand::Backspace))),
		"glorp editor edit delete-forward" => ensure_no_input(path, input)
			.map(|()| GlorpCommand::Editor(EditorCommand::Edit(EditorEditCommand::DeleteForward))),
		"glorp editor edit delete-selection" => ensure_no_input(path, input)
			.map(|()| GlorpCommand::Editor(EditorCommand::Edit(EditorEditCommand::DeleteSelection))),
		"glorp editor history" => decode_editor_history(input),
		"glorp ui sidebar select" => decode_ui_sidebar_select(input),
		"glorp ui viewport scroll-to" => decode_ui_viewport_scroll_to(input),
		"glorp ui pane-ratio-set" => decode_ui_pane_ratio_set(input),
		"glorp scene ensure" => ensure_no_input(path, input).map(|()| GlorpCommand::Scene(SceneCommand::Ensure)),
		"glorp txn" => decode_txn(input).map(GlorpCommand::Txn),
		_ => Err(GlorpError::validation(None, format!("unknown command path `{path}`"))),
	}
}

pub fn query_invocation(path: &str, input: Option<&GlorpValue>) -> Result<GlorpQuery, GlorpError> {
	match path {
		"glorp schema" => ensure_no_input(path, input).map(|()| GlorpQuery::Schema),
		"glorp get config" => ensure_no_input(path, input).map(|()| GlorpQuery::Config),
		"glorp get state" => ensure_no_input(path, input).map(|()| GlorpQuery::Snapshot {
			scene: crate::SceneLevel::Materialize,
			include_document_text: true,
		}),
		"glorp get document-text" => ensure_no_input(path, input).map(|()| GlorpQuery::DocumentText),
		"glorp get selection" => ensure_no_input(path, input).map(|()| GlorpQuery::Selection),
		"glorp get inspect-details" => decode_get_inspect_details(input),
		"glorp get perf" => ensure_no_input(path, input).map(|()| GlorpQuery::PerfDashboard),
		"glorp get ui" => ensure_no_input(path, input).map(|()| GlorpQuery::UiState),
		"glorp get capabilities" => ensure_no_input(path, input).map(|()| GlorpQuery::Capabilities),
		_ => Err(GlorpError::validation(None, format!("unknown query path `{path}`"))),
	}
}

pub fn completion_values(name: &str) -> Option<&'static [&'static str]> {
	match name {
		"motion" => Some(<crate::EditorMotion as crate::EnumValue>::allowed_values()),
		"mode" => Some(<crate::EditorModeCommand as crate::EnumValue>::allowed_values()),
		"history" => Some(<crate::EditorHistoryCommand as crate::EnumValue>::allowed_values()),
		"tab" => Some(<crate::SidebarTab as crate::EnumValue>::allowed_values()),
		_ => None,
	}
}

pub fn render_nu_completions() -> String {
	["motion", "mode", "history", "tab"]
		.into_iter()
		.filter_map(|name| completion_values(name).map(|values| (name, values)))
		.map(|(name, values)| {
			let values = values
				.iter()
				.map(|value| format!("\"{value}\""))
				.collect::<Vec<_>>()
				.join(" ");
			format!("export def \"nu-complete glorp {name}\" [] {{ [{values}] }}\n")
		})
		.collect()
}

pub fn render_nu_module() -> String {
	let mut module = String::from("plugin use glorp\nuse ./completions.nu *\n\n");
	module.push_str(&render_nu_command_helpers());
	module.push('\n');
	module.push_str(&render_nu_aliases());
	module
}

fn render_nu_command_helpers() -> String {
	surface_specs()
		.into_iter()
		.filter(|spec| matches!(spec.kind, SurfaceKind::Command) && spec.path != "glorp txn")
		.map(render_nu_command_helper)
		.collect::<Vec<_>>()
		.join("\n")
}

fn render_nu_command_helper(spec: SurfaceSpec) -> String {
	let helper_name = spec.path.replacen("glorp ", "glorp cmd ", 1);
	let signature = spec
		.args
		.iter()
		.map(|arg| {
			let mut shape = match arg.kind {
				SurfaceArgKind::String => "string".to_owned(),
				SurfaceArgKind::Int => "int".to_owned(),
				SurfaceArgKind::Float => "number".to_owned(),
				SurfaceArgKind::Bool => "bool".to_owned(),
				SurfaceArgKind::Any => "any".to_owned(),
			};
			if let Some(completion) = arg.completion {
				shape.push_str(&format!("@\"nu-complete glorp {completion}\""));
			}
			let suffix = if arg.required { "" } else { "?" };
			format!("{}{}: {shape}", arg.name, suffix)
		})
		.collect::<Vec<_>>()
		.join(" ");
	let input = if spec.args.is_empty() {
		"null".to_owned()
	} else {
		let fields = spec
			.args
			.iter()
			.map(|arg| format!("{}: ${}", arg.name, arg.name))
			.collect::<Vec<_>>()
			.join(" ");
		format!("{{{fields}}}")
	};
	format!(
		"export def \"{helper_name}\" [{signature}] {{\n  {{path: \"{}\" input: {input}}}\n}}\n",
		spec.path
	)
}

fn render_nu_aliases() -> String {
	[
		render_alias("glorp open-inspect", "glorp ui sidebar select inspect"),
		render_alias("glorp open-perf", "glorp ui sidebar select perf"),
		render_alias("glorp scroll-to-top", "glorp ui viewport scroll-to 0 0"),
		render_alias("glorp undo", "glorp editor history undo"),
		render_alias("glorp redo", "glorp editor history redo"),
	]
	.join("\n")
}

fn render_alias(name: &str, target: &str) -> String {
	format!("export def \"{name}\" [] {{\n  {target}\n}}\n")
}

fn decode_txn(input: Option<&GlorpValue>) -> Result<GlorpTxn, GlorpError> {
	let fields = record_fields("glorp txn", input)?;
	let commands = list_field(fields, "commands")?
		.into_iter()
		.map(invocation_from_value)
		.collect::<Result<Vec<_>, _>>()?;
	Ok(GlorpTxn { commands })
}

fn invocation_from_value(value: &GlorpValue) -> Result<GlorpInvocation, GlorpError> {
	let fields = value.as_record().ok_or_else(|| {
		GlorpError::validation(
			None,
			format!("transaction command must be a record, got {}", value.kind()),
		)
	})?;
	let path = string_field(fields, "path")?.to_owned();
	let input = fields.get("input").cloned();
	Ok(GlorpInvocation { path, input })
}

fn decode_config_set(input: Option<&GlorpValue>) -> Result<GlorpCommand, GlorpError> {
	let fields = record_fields("glorp config set", input)?;
	Ok(GlorpCommand::Config(ConfigCommand::Set {
		path: string_field(fields, "path")?.to_owned(),
		value: value_field(fields, "value")?.clone(),
	}))
}

fn decode_config_reset(input: Option<&GlorpValue>) -> Result<GlorpCommand, GlorpError> {
	let fields = record_fields("glorp config reset", input)?;
	Ok(GlorpCommand::Config(ConfigCommand::Reset {
		path: string_field(fields, "path")?.to_owned(),
	}))
}

fn decode_config_patch(input: Option<&GlorpValue>) -> Result<GlorpCommand, GlorpError> {
	let fields = record_fields("glorp config patch", input)?;
	let patch = value_field(fields, "patch")?;
	Ok(GlorpCommand::Config(ConfigCommand::Patch {
		values: flatten_patch(patch)?,
	}))
}

fn decode_document_replace(input: Option<&GlorpValue>) -> Result<GlorpCommand, GlorpError> {
	let fields = record_fields("glorp doc replace", input)?;
	Ok(GlorpCommand::Document(DocumentCommand::Replace {
		text: string_field(fields, "text")?.to_owned(),
	}))
}

fn decode_editor_motion(input: Option<&GlorpValue>) -> Result<GlorpCommand, GlorpError> {
	let fields = record_fields("glorp editor motion", input)?;
	Ok(GlorpCommand::Editor(EditorCommand::Motion(parse_enum_field(
		fields, "motion",
	)?)))
}

fn decode_editor_mode(input: Option<&GlorpValue>) -> Result<GlorpCommand, GlorpError> {
	let fields = record_fields("glorp editor mode", input)?;
	Ok(GlorpCommand::Editor(EditorCommand::Mode(parse_enum_field(
		fields, "mode",
	)?)))
}

fn decode_editor_edit_insert(input: Option<&GlorpValue>) -> Result<GlorpCommand, GlorpError> {
	let fields = record_fields("glorp editor edit insert", input)?;
	Ok(GlorpCommand::Editor(EditorCommand::Edit(EditorEditCommand::Insert {
		text: string_field(fields, "text")?.to_owned(),
	})))
}

fn decode_editor_history(input: Option<&GlorpValue>) -> Result<GlorpCommand, GlorpError> {
	let fields = record_fields("glorp editor history", input)?;
	Ok(GlorpCommand::Editor(EditorCommand::History(parse_enum_field(
		fields, "action",
	)?)))
}

fn decode_ui_sidebar_select(input: Option<&GlorpValue>) -> Result<GlorpCommand, GlorpError> {
	let fields = record_fields("glorp ui sidebar select", input)?;
	Ok(GlorpCommand::Ui(UiCommand::SidebarSelect {
		tab: parse_enum_field(fields, "tab")?,
	}))
}

fn decode_ui_viewport_scroll_to(input: Option<&GlorpValue>) -> Result<GlorpCommand, GlorpError> {
	let fields = record_fields("glorp ui viewport scroll-to", input)?;
	Ok(GlorpCommand::Ui(UiCommand::ViewportScrollTo {
		x: float_field(fields, "x")? as f32,
		y: float_field(fields, "y")? as f32,
	}))
}

fn decode_ui_pane_ratio_set(input: Option<&GlorpValue>) -> Result<GlorpCommand, GlorpError> {
	let fields = record_fields("glorp ui pane-ratio-set", input)?;
	Ok(GlorpCommand::Ui(UiCommand::PaneRatioSet {
		ratio: float_field(fields, "ratio")? as f32,
	}))
}

fn decode_get_inspect_details(input: Option<&GlorpValue>) -> Result<GlorpQuery, GlorpError> {
	let target = input.and_then(|value| {
		value
			.as_record()
			.and_then(|fields| fields.get("target"))
			.and_then(GlorpValue::as_str)
	});
	Ok(GlorpQuery::InspectDetails {
		target: target.map(parse_canvas_target).transpose()?,
	})
}

fn flatten_patch(value: &GlorpValue) -> Result<Vec<ConfigAssignment>, GlorpError> {
	let mut assignments = Vec::new();
	let mut path = String::new();
	flatten_patch_into(&mut assignments, &mut path, value)?;
	Ok(assignments)
}

fn flatten_patch_into(
	assignments: &mut Vec<ConfigAssignment>, path: &mut String, value: &GlorpValue,
) -> Result<(), GlorpError> {
	match value {
		GlorpValue::Record(fields) => fields.iter().try_for_each(|(key, value)| {
			let len = path.len();
			if !path.is_empty() {
				path.push('.');
			}
			path.push_str(key);
			let result = flatten_patch_into(assignments, path, value);
			path.truncate(len);
			result
		}),
		other => {
			assignments.push(ConfigAssignment {
				path: path.clone(),
				value: other.clone(),
			});
			Ok(())
		}
	}
}

fn parse_canvas_target(value: &str) -> Result<CanvasTarget, GlorpError> {
	let (kind, index) = value
		.split_once(':')
		.ok_or_else(|| GlorpError::validation(None, format!("invalid canvas target `{value}`")))?;
	let index = index
		.parse::<usize>()
		.map_err(|error| GlorpError::validation(None, format!("invalid canvas target `{value}`: {error}")))?;
	match kind {
		"run" => Ok(CanvasTarget::Run(index)),
		"cluster" => Ok(CanvasTarget::Cluster(index)),
		_ => Err(GlorpError::validation(
			None,
			format!("unknown canvas target kind `{kind}`"),
		)),
	}
}

fn ensure_no_input(path: &str, input: Option<&GlorpValue>) -> Result<(), GlorpError> {
	match input {
		None | Some(GlorpValue::Null) => Ok(()),
		Some(value) if value.as_record().is_some_and(BTreeMap::is_empty) => Ok(()),
		Some(value) => Err(GlorpError::validation(
			None,
			format!("`{path}` does not accept input, got {}", value.kind()),
		)),
	}
}

fn record_fields<'a>(
	path: &str, input: Option<&'a GlorpValue>,
) -> Result<&'a BTreeMap<String, GlorpValue>, GlorpError> {
	let value = input.ok_or_else(|| GlorpError::validation(None, format!("`{path}` requires input")))?;
	value
		.as_record()
		.ok_or_else(|| GlorpError::validation(None, format!("`{path}` requires a record input")))
}

fn value_field<'a>(fields: &'a BTreeMap<String, GlorpValue>, name: &str) -> Result<&'a GlorpValue, GlorpError> {
	fields
		.get(name)
		.ok_or_else(|| GlorpError::validation(None, format!("missing field `{name}`")))
}

fn string_field<'a>(fields: &'a BTreeMap<String, GlorpValue>, name: &str) -> Result<&'a str, GlorpError> {
	value_field(fields, name)?
		.as_str()
		.ok_or_else(|| GlorpError::validation(None, format!("field `{name}` must be a string")))
}

fn list_field<'a>(fields: &'a BTreeMap<String, GlorpValue>, name: &str) -> Result<&'a [GlorpValue], GlorpError> {
	match value_field(fields, name)? {
		GlorpValue::List(values) => Ok(values.as_slice()),
		_ => Err(GlorpError::validation(None, format!("field `{name}` must be a list"))),
	}
}

fn float_field(fields: &BTreeMap<String, GlorpValue>, name: &str) -> Result<f64, GlorpError> {
	value_field(fields, name)?
		.as_f64()
		.ok_or_else(|| GlorpError::validation(None, format!("field `{name}` must be numeric")))
}

fn parse_enum_field<T>(fields: &BTreeMap<String, GlorpValue>, name: &str) -> Result<T, GlorpError>
where
	T: crate::EnumValue, {
	let value = string_field(fields, name)?;
	T::parse(value).ok_or_else(|| {
		GlorpError::validation_with_allowed(
			None,
			format!("invalid value `{value}` for `{name}`"),
			T::allowed_values().iter().copied().map(str::to_owned).collect(),
		)
	})
}

fn arg(
	name: &'static str, docs: &'static str, kind: SurfaceArgKind, required: bool, completion: Option<&'static str>,
) -> SurfaceArg {
	SurfaceArg {
		name,
		docs,
		kind,
		required,
		completion,
	}
}

fn command(
	path: &'static str, docs: &'static str, input: Option<TypeRef>, args: Vec<SurfaceArg>, output: TypeRef,
) -> SurfaceSpec {
	SurfaceSpec {
		path,
		docs,
		kind: SurfaceKind::Command,
		input,
		args,
		output,
	}
}

fn query(
	path: &'static str, docs: &'static str, input: Option<TypeRef>, args: Vec<SurfaceArg>, output: TypeRef,
) -> SurfaceSpec {
	SurfaceSpec {
		path,
		docs,
		kind: SurfaceKind::Query,
		input,
		args,
		output,
	}
}

fn helper(
	path: &'static str, docs: &'static str, kind: HelperKind, input: Option<TypeRef>, args: Vec<SurfaceArg>,
	output: TypeRef,
) -> SurfaceSpec {
	SurfaceSpec {
		path,
		docs,
		kind: SurfaceKind::Helper(kind),
		input,
		args,
		output,
	}
}

const fn built(kind: BuiltinType) -> TypeRef {
	TypeRef::Builtin(kind)
}

fn named(name: &str) -> TypeRef {
	TypeRef::Named(name.to_owned())
}
