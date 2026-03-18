use {
	crate::{
		BuiltinType, CanvasTarget, CommandSchema, ConfigAssignment, ConfigCommand, DocumentCommand, EditorCommand,
		EditorEditCommand, GlorpCommand, GlorpError, GlorpQuery, GlorpTxn, GlorpValue, HelperKind, HelperSchema,
		QuerySchema, SceneCommand, TypeRef, UiCommand,
	},
	std::collections::BTreeMap,
};

pub type CommandBuilder = fn(Option<&GlorpValue>) -> Result<GlorpCommand, GlorpError>;
pub type QueryBuilder = fn(Option<&GlorpValue>) -> Result<GlorpQuery, GlorpError>;

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
	pub args: Vec<SurfaceArg>,
	pub input: Option<TypeRef>,
	pub output: TypeRef,
}

#[derive(Debug, Clone)]
pub struct CommandSpec {
	pub surface: SurfaceSpec,
	pub build: CommandBuilder,
	pub builder: bool,
}

#[derive(Debug, Clone)]
pub struct QuerySpec {
	pub surface: SurfaceSpec,
	pub build: QueryBuilder,
}

#[derive(Debug, Clone)]
pub struct HelperSpec {
	pub surface: SurfaceSpec,
	pub kind: HelperKind,
}

pub fn command_specs() -> Vec<CommandSpec> {
	vec![
		command(
			"glorp config set",
			"Set one config field.",
			Some(named("ConfigAssignment")),
			vec![
				arg("path", "Config path.", SurfaceArgKind::String, true, None),
				arg("value", "Config value.", SurfaceArgKind::Any, true, None),
			],
			named("GlorpOutcome"),
			decode_config_set,
			true,
		),
		command(
			"glorp config reset",
			"Reset one config field to its default.",
			Some(named("ConfigPathInput")),
			vec![arg("path", "Config path.", SurfaceArgKind::String, true, None)],
			named("GlorpOutcome"),
			decode_config_reset,
			true,
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
			decode_config_patch,
			true,
		),
		command(
			"glorp config reload",
			"Reload the durable config file.",
			None,
			vec![],
			named("GlorpOutcome"),
			decode_config_reload,
			true,
		),
		command(
			"glorp config persist",
			"Persist the effective config to the durable config file.",
			None,
			vec![],
			named("GlorpOutcome"),
			decode_config_persist,
			true,
		),
		command(
			"glorp doc replace",
			"Replace the document text.",
			Some(named("TextInput")),
			vec![arg("text", "Document text.", SurfaceArgKind::String, true, None)],
			named("GlorpOutcome"),
			decode_document_replace,
			true,
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
			decode_editor_motion,
			true,
		),
		command(
			"glorp editor mode",
			"Apply a typed mode command.",
			Some(named("EditorModeInput")),
			vec![arg("mode", "Mode command.", SurfaceArgKind::String, true, Some("mode"))],
			named("GlorpOutcome"),
			decode_editor_mode,
			true,
		),
		command(
			"glorp editor edit insert",
			"Insert text.",
			Some(named("TextInput")),
			vec![arg("text", "Text to insert.", SurfaceArgKind::String, true, None)],
			named("GlorpOutcome"),
			decode_editor_edit_insert,
			true,
		),
		command(
			"glorp editor edit backspace",
			"Delete backward.",
			None,
			vec![],
			named("GlorpOutcome"),
			decode_editor_edit_backspace,
			true,
		),
		command(
			"glorp editor edit delete-forward",
			"Delete forward.",
			None,
			vec![],
			named("GlorpOutcome"),
			decode_editor_edit_delete_forward,
			true,
		),
		command(
			"glorp editor edit delete-selection",
			"Delete the current selection.",
			None,
			vec![],
			named("GlorpOutcome"),
			decode_editor_edit_delete_selection,
			true,
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
			decode_editor_history,
			true,
		),
		command(
			"glorp ui sidebar select",
			"Select a sidebar tab.",
			Some(named("SidebarTabInput")),
			vec![arg("tab", "Sidebar tab.", SurfaceArgKind::String, true, Some("tab"))],
			named("GlorpOutcome"),
			decode_ui_sidebar_select,
			true,
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
			decode_ui_viewport_scroll_to,
			true,
		),
		command(
			"glorp ui pane-ratio-set",
			"Set the sidebar/canvas split ratio.",
			Some(named("PaneRatioInput")),
			vec![arg("ratio", "Pane ratio.", SurfaceArgKind::Float, true, None)],
			named("GlorpOutcome"),
			decode_ui_pane_ratio_set,
			true,
		),
		command(
			"glorp scene ensure",
			"Materialize scene state.",
			None,
			vec![],
			named("GlorpOutcome"),
			decode_scene_ensure,
			true,
		),
		command(
			"glorp txn",
			"Apply a transaction atomically.",
			Some(named("GlorpTxn")),
			vec![arg(
				"commands",
				"Ordered typed command values.",
				SurfaceArgKind::Any,
				true,
				None,
			)],
			named("GlorpOutcome"),
			decode_txn_command,
			false,
		),
	]
}

pub fn query_specs() -> Vec<QuerySpec> {
	vec![
		query(
			"glorp schema",
			"Return the runtime reflection schema.",
			None,
			vec![],
			named("GlorpSchema"),
			decode_schema_query,
		),
		query(
			"glorp get config",
			"Return the effective runtime config.",
			None,
			vec![],
			named("GlorpConfig"),
			decode_get_config,
		),
		query(
			"glorp get state",
			"Return a snapshot of runtime state.",
			None,
			vec![],
			named("GlorpSnapshot"),
			decode_get_state,
		),
		query(
			"glorp get document-text",
			"Return the current document text.",
			None,
			vec![],
			built(BuiltinType::String),
			decode_get_document_text,
		),
		query(
			"glorp get selection",
			"Return the current selection read model.",
			None,
			vec![],
			named("SelectionStateView"),
			decode_get_selection,
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
			decode_get_inspect_details,
		),
		query(
			"glorp get perf",
			"Return the runtime perf dashboard.",
			None,
			vec![],
			named("PerfDashboardView"),
			decode_get_perf,
		),
		query(
			"glorp get ui",
			"Return the current UI state.",
			None,
			vec![],
			named("UiStateView"),
			decode_get_ui,
		),
		query(
			"glorp get capabilities",
			"Return stable runtime capability flags.",
			None,
			vec![],
			named("GlorpCapabilities"),
			decode_get_capabilities,
		),
	]
}

pub fn helper_specs() -> Vec<HelperSpec> {
	vec![
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
	]
}

pub fn command_schemas() -> Vec<CommandSchema> {
	command_specs()
		.into_iter()
		.map(|spec| CommandSchema {
			path: spec.surface.path.to_owned(),
			docs: spec.surface.docs.to_owned(),
			input: spec.surface.input.unwrap_or_else(|| built(BuiltinType::Null)),
			output: spec.surface.output,
		})
		.collect()
}

pub fn query_schemas() -> Vec<QuerySchema> {
	query_specs()
		.into_iter()
		.map(|spec| QuerySchema {
			path: spec.surface.path.to_owned(),
			docs: spec.surface.docs.to_owned(),
			input: spec.surface.input,
			output: spec.surface.output,
		})
		.collect()
}

pub fn helper_schemas() -> Vec<HelperSchema> {
	helper_specs()
		.into_iter()
		.map(|spec| HelperSchema {
			path: spec.surface.path.to_owned(),
			docs: spec.surface.docs.to_owned(),
			kind: spec.kind,
			input: spec.surface.input,
			output: spec.surface.output,
		})
		.collect()
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
	module.push_str(&render_nu_aliases());
	module
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

fn decode_schema_query(input: Option<&GlorpValue>) -> Result<GlorpQuery, GlorpError> {
	ensure_no_input("glorp schema", input).map(|()| GlorpQuery::Schema)
}

fn decode_get_config(input: Option<&GlorpValue>) -> Result<GlorpQuery, GlorpError> {
	ensure_no_input("glorp get config", input).map(|()| GlorpQuery::Config)
}

fn decode_get_state(input: Option<&GlorpValue>) -> Result<GlorpQuery, GlorpError> {
	ensure_no_input("glorp get state", input).map(|()| GlorpQuery::Snapshot {
		scene: crate::SceneLevel::Materialize,
		include_document_text: true,
	})
}

fn decode_get_document_text(input: Option<&GlorpValue>) -> Result<GlorpQuery, GlorpError> {
	ensure_no_input("glorp get document-text", input).map(|()| GlorpQuery::DocumentText)
}

fn decode_get_selection(input: Option<&GlorpValue>) -> Result<GlorpQuery, GlorpError> {
	ensure_no_input("glorp get selection", input).map(|()| GlorpQuery::Selection)
}

fn decode_get_perf(input: Option<&GlorpValue>) -> Result<GlorpQuery, GlorpError> {
	ensure_no_input("glorp get perf", input).map(|()| GlorpQuery::PerfDashboard)
}

fn decode_get_ui(input: Option<&GlorpValue>) -> Result<GlorpQuery, GlorpError> {
	ensure_no_input("glorp get ui", input).map(|()| GlorpQuery::UiState)
}

fn decode_get_capabilities(input: Option<&GlorpValue>) -> Result<GlorpQuery, GlorpError> {
	ensure_no_input("glorp get capabilities", input).map(|()| GlorpQuery::Capabilities)
}

fn decode_txn_command(input: Option<&GlorpValue>) -> Result<GlorpCommand, GlorpError> {
	decode_txn(input).map(GlorpCommand::Txn)
}

fn decode_txn(input: Option<&GlorpValue>) -> Result<GlorpTxn, GlorpError> {
	let fields = record_fields("glorp txn", input)?;
	let commands = list_field(fields, "commands")?
		.iter()
		.cloned()
		.map(|value| serde_json::from_value::<GlorpCommand>(value.into()))
		.collect::<Result<Vec<_>, _>>()
		.map_err(|error| GlorpError::validation(None, format!("invalid typed transaction command: {error}")))?;
	Ok(GlorpTxn { commands })
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

fn decode_config_reload(input: Option<&GlorpValue>) -> Result<GlorpCommand, GlorpError> {
	ensure_no_input("glorp config reload", input).map(|()| GlorpCommand::Config(ConfigCommand::Reload))
}

fn decode_config_persist(input: Option<&GlorpValue>) -> Result<GlorpCommand, GlorpError> {
	ensure_no_input("glorp config persist", input).map(|()| GlorpCommand::Config(ConfigCommand::Persist))
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

fn decode_editor_edit_backspace(input: Option<&GlorpValue>) -> Result<GlorpCommand, GlorpError> {
	ensure_no_input("glorp editor edit backspace", input)
		.map(|()| GlorpCommand::Editor(EditorCommand::Edit(EditorEditCommand::Backspace)))
}

fn decode_editor_edit_delete_forward(input: Option<&GlorpValue>) -> Result<GlorpCommand, GlorpError> {
	ensure_no_input("glorp editor edit delete-forward", input)
		.map(|()| GlorpCommand::Editor(EditorCommand::Edit(EditorEditCommand::DeleteForward)))
}

fn decode_editor_edit_delete_selection(input: Option<&GlorpValue>) -> Result<GlorpCommand, GlorpError> {
	ensure_no_input("glorp editor edit delete-selection", input)
		.map(|()| GlorpCommand::Editor(EditorCommand::Edit(EditorEditCommand::DeleteSelection)))
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

fn decode_scene_ensure(input: Option<&GlorpValue>) -> Result<GlorpCommand, GlorpError> {
	ensure_no_input("glorp scene ensure", input).map(|()| GlorpCommand::Scene(SceneCommand::Ensure))
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

pub fn flatten_patch(value: &GlorpValue) -> Result<Vec<ConfigAssignment>, GlorpError> {
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

pub fn parse_canvas_target(value: &str) -> Result<CanvasTarget, GlorpError> {
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

fn float_field(fields: &BTreeMap<String, GlorpValue>, name: &str) -> Result<f64, GlorpError> {
	value_field(fields, name)?
		.as_f64()
		.ok_or_else(|| GlorpError::validation(None, format!("field `{name}` must be a float")))
}

fn list_field<'a>(fields: &'a BTreeMap<String, GlorpValue>, name: &str) -> Result<&'a [GlorpValue], GlorpError> {
	match value_field(fields, name)? {
		GlorpValue::List(values) => Ok(values),
		_ => Err(GlorpError::validation(None, format!("field `{name}` must be a list"))),
	}
}

fn parse_enum_field<T>(fields: &BTreeMap<String, GlorpValue>, name: &str) -> Result<T, GlorpError>
where
	T: crate::EnumValue, {
	let value = string_field(fields, name)?;
	T::parse(value).ok_or_else(|| {
		GlorpError::validation_with_allowed(
			None,
			format!("invalid value `{value}` for field `{name}`"),
			T::allowed_values().iter().copied().map(str::to_owned).collect(),
		)
	})
}

fn command(
	path: &'static str, docs: &'static str, input: Option<TypeRef>, args: Vec<SurfaceArg>, output: TypeRef,
	build: CommandBuilder, builder: bool,
) -> CommandSpec {
	CommandSpec {
		surface: SurfaceSpec {
			path,
			docs,
			args,
			input,
			output,
		},
		build,
		builder,
	}
}

fn query(
	path: &'static str, docs: &'static str, input: Option<TypeRef>, args: Vec<SurfaceArg>, output: TypeRef,
	build: QueryBuilder,
) -> QuerySpec {
	QuerySpec {
		surface: SurfaceSpec {
			path,
			docs,
			args,
			input,
			output,
		},
		build,
	}
}

fn helper(
	path: &'static str, docs: &'static str, kind: HelperKind, input: Option<TypeRef>, args: Vec<SurfaceArg>,
	output: TypeRef,
) -> HelperSpec {
	HelperSpec {
		surface: SurfaceSpec {
			path,
			docs,
			args,
			input,
			output,
		},
		kind,
	}
}

const fn arg(
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

fn built(ty: BuiltinType) -> TypeRef {
	TypeRef::Builtin(ty)
}

fn named(name: &str) -> TypeRef {
	TypeRef::Named(name.to_owned())
}
