use crate::*;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct GlorpSchema {
	pub version: u32,
	pub named_types: Vec<NamedTypeSchema>,
	pub config: Vec<ConfigFieldSchema>,
	pub commands: Vec<CommandSchema>,
	pub queries: Vec<QuerySchema>,
	pub events: Vec<EventSchema>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct NamedTypeSchema {
	pub name: String,
	pub docs: String,
	pub kind: TypeSchema,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct ConfigFieldSchema {
	pub path: ConfigPath,
	pub docs: String,
	pub ty: TypeRef,
	pub default: GlorpValue,
	pub mutable: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct CommandSchema {
	pub path: String,
	pub docs: String,
	pub input: TypeRef,
	pub output: TypeRef,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct QuerySchema {
	pub path: String,
	pub docs: String,
	pub output: TypeRef,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct EventSchema {
	pub path: String,
	pub docs: String,
	pub payload: TypeRef,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum TypeRef {
	Builtin(BuiltinType),
	Named(String),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum BuiltinType {
	Null,
	Bool,
	Int,
	Float,
	String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum TypeSchema {
	Enum { variants: Vec<EnumVariantSchema> },
	Record { fields: Vec<FieldSchema> },
	List { item: TypeRef },
	Option { item: TypeRef },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct FieldSchema {
	pub name: String,
	pub docs: String,
	pub ty: TypeRef,
	pub required: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct EnumVariantSchema {
	pub name: String,
	pub docs: String,
}

#[must_use]
pub fn glorp_schema() -> GlorpSchema {
	GlorpSchema {
		version: 1,
		named_types: vec![
			enum_type(
				"SamplePreset",
				"Built-in sample document presets.",
				&[
					("tall", "A tall multi-script sample."),
					("mixed", "A short mixed-script sample."),
					("rust", "A Rust source sample."),
					("ligatures", "A ligature-heavy sample."),
					("arabic", "An Arabic sample."),
					("cjk", "A CJK sample."),
					("emoji", "An emoji-heavy sample."),
					("custom", "No built-in sample."),
				],
			),
			enum_type(
				"WrapChoice",
				"Stable editor wrapping choices.",
				&[
					("none", "Do not wrap lines."),
					("word", "Wrap at word boundaries."),
					("glyph", "Wrap at glyph boundaries."),
					(
						"word-or-glyph",
						"Prefer word boundaries, fall back to glyph boundaries.",
					),
				],
			),
			enum_type(
				"FontChoice",
				"Stable editor font families.",
				&[
					("jetbrains-mono", "JetBrains Mono."),
					("monospace", "The platform monospace family."),
					("noto-sans-cjk", "Noto Sans CJK."),
					("sans-serif", "The platform sans-serif family."),
				],
			),
			enum_type(
				"ShapingChoice",
				"Stable shaping choices.",
				&[
					("auto", "Choose shaping based on content."),
					("basic", "Use basic shaping."),
					("advanced", "Use advanced shaping."),
				],
			),
			enum_type(
				"SidebarTab",
				"Stable sidebar tabs.",
				&[
					("controls", "Configuration controls."),
					("inspect", "Scene inspection."),
					("perf", "Performance projections."),
				],
			),
			enum_type(
				"EditorMotion",
				"Typed editor motions.",
				&[
					("left", "Move left."),
					("right", "Move right."),
					("up", "Move up."),
					("down", "Move down."),
					("line-start", "Move to line start."),
					("line-end", "Move to line end."),
				],
			),
			enum_type(
				"EditorModeCommand",
				"Typed mode transitions.",
				&[
					("enter-insert-before", "Enter insert mode before the selection."),
					("enter-insert-after", "Enter insert mode after the selection."),
					("exit-insert", "Return to normal mode."),
				],
			),
			enum_type(
				"EditorHistoryCommand",
				"Typed undo/redo operations.",
				&[
					("undo", "Undo the most recent edit."),
					("redo", "Redo the most recent undone edit."),
				],
			),
			enum_type(
				"EditorMode",
				"Stable editor modes.",
				&[("normal", "Normal mode."), ("insert", "Insert mode.")],
			),
			enum_type(
				"GlorpCommand",
				"Top-level command namespaces.",
				&[
					("txn", "Atomic transaction."),
					("config", "Config mutation."),
					("document", "Document mutation."),
					("editor", "Editor command."),
					("ui", "UI command."),
					("scene", "Scene command."),
				],
			),
			record_type(
				"ConfigAssignment",
				"One path-based config assignment.",
				vec![
					field("path", "Config path.", built(BuiltinType::String), true),
					field("value", "Config value.", built(BuiltinType::String), true),
				],
			),
			record_type(
				"ConfigPatch",
				"Multiple path-based config assignments.",
				vec![field(
					"values",
					"Config assignments.",
					TypeRef::Named("ConfigAssignmentList".to_owned()),
					true,
				)],
			),
			record_type(
				"ScrollTarget",
				"Viewport scroll target.",
				vec![
					field("x", "Horizontal scroll offset.", built(BuiltinType::Float), true),
					field("y", "Vertical scroll offset.", built(BuiltinType::Float), true),
				],
			),
			record_type(
				"GlorpCapabilities",
				"Stable runtime capability flags.",
				vec![
					field(
						"transactions",
						"Whether transactions are supported.",
						built(BuiltinType::Bool),
						true,
					),
					field(
						"subscriptions",
						"Whether subscriptions are supported.",
						built(BuiltinType::Bool),
						true,
					),
					field(
						"transports",
						"Supported transport names.",
						TypeRef::Named("StringList".to_owned()),
						true,
					),
				],
			),
			record_type(
				"GlorpOutcome",
				"Revisioned outcome for a successful command.",
				vec![
					field("delta", "Change flags.", TypeRef::Named("GlorpDelta".to_owned()), true),
					field(
						"revisions",
						"Post-command revisions.",
						TypeRef::Named("GlorpRevisions".to_owned()),
						true,
					),
					field(
						"changed_config_paths",
						"Config paths changed by the command.",
						TypeRef::Named("StringList".to_owned()),
						true,
					),
				],
			),
			record_type(
				"GlorpDelta",
				"Boolean change flags.",
				vec![
					field("text_changed", "Document text changed.", built(BuiltinType::Bool), true),
					field("view_changed", "Editor view changed.", built(BuiltinType::Bool), true),
					field(
						"selection_changed",
						"Selection changed.",
						built(BuiltinType::Bool),
						true,
					),
					field("mode_changed", "Mode changed.", built(BuiltinType::Bool), true),
					field("config_changed", "Config changed.", built(BuiltinType::Bool), true),
					field("ui_changed", "UI state changed.", built(BuiltinType::Bool), true),
					field("scene_changed", "Scene state changed.", built(BuiltinType::Bool), true),
				],
			),
			record_type(
				"GlorpRevisions",
				"Runtime revisions.",
				vec![
					field("editor", "Editor revision.", built(BuiltinType::Int), true),
					field(
						"scene",
						"Scene revision if materialized.",
						built(BuiltinType::Int),
						false,
					),
					field("config", "Config revision.", built(BuiltinType::Int), true),
				],
			),
			record_type(
				"GlorpConfig",
				"Stable runtime config.",
				vec![
					field(
						"editor",
						"Editor config namespace.",
						TypeRef::Named("EditorConfig".to_owned()),
						true,
					),
					field(
						"inspect",
						"Inspect config namespace.",
						TypeRef::Named("InspectConfig".to_owned()),
						true,
					),
				],
			),
			record_type(
				"EditorConfig",
				"Editor configuration namespace.",
				vec![
					field(
						"preset",
						"Sample preset.",
						TypeRef::Named("SamplePreset".to_owned()),
						false,
					),
					field("font", "Font choice.", TypeRef::Named("FontChoice".to_owned()), true),
					field(
						"shaping",
						"Shaping choice.",
						TypeRef::Named("ShapingChoice".to_owned()),
						true,
					),
					field(
						"wrapping",
						"Wrapping choice.",
						TypeRef::Named("WrapChoice".to_owned()),
						true,
					),
					field("font_size", "Font size.", built(BuiltinType::Float), true),
					field("line_height", "Line height.", built(BuiltinType::Float), true),
				],
			),
			record_type(
				"InspectConfig",
				"Inspect configuration namespace.",
				vec![
					field("show_baselines", "Show baselines.", built(BuiltinType::Bool), true),
					field("show_hitboxes", "Show hitboxes.", built(BuiltinType::Bool), true),
				],
			),
			record_type(
				"GlorpSnapshot",
				"Read-only runtime snapshot.",
				vec![
					field(
						"revisions",
						"Runtime revisions.",
						TypeRef::Named("GlorpRevisions".to_owned()),
						true,
					),
					field(
						"config",
						"Effective config.",
						TypeRef::Named("GlorpConfig".to_owned()),
						true,
					),
					field(
						"editor",
						"Editor state view.",
						TypeRef::Named("EditorStateView".to_owned()),
						true,
					),
					field(
						"scene",
						"Scene state view.",
						TypeRef::Named("SceneStateView".to_owned()),
						false,
					),
					field("ui", "UI state view.", TypeRef::Named("UiStateView".to_owned()), true),
				],
			),
			record_type(
				"EditorStateView",
				"Stable editor state view.",
				vec![
					field("mode", "Editor mode.", TypeRef::Named("EditorMode".to_owned()), true),
					field("text_bytes", "Document size in bytes.", built(BuiltinType::Int), true),
					field("undo_depth", "Undo depth.", built(BuiltinType::Int), true),
					field("redo_depth", "Redo depth.", built(BuiltinType::Int), true),
				],
			),
			record_type(
				"SceneStateView",
				"Stable scene state view.",
				vec![
					field("revision", "Scene revision.", built(BuiltinType::Int), true),
					field("measured_width", "Measured width.", built(BuiltinType::Float), true),
					field("measured_height", "Measured height.", built(BuiltinType::Float), true),
				],
			),
			record_type(
				"UiStateView",
				"Stable UI state view.",
				vec![
					field(
						"active_tab",
						"Active sidebar tab.",
						TypeRef::Named("SidebarTab".to_owned()),
						true,
					),
					field(
						"canvas_scroll_x",
						"Horizontal scroll offset.",
						built(BuiltinType::Float),
						true,
					),
					field(
						"canvas_scroll_y",
						"Vertical scroll offset.",
						built(BuiltinType::Float),
						true,
					),
					field("pane_ratio", "Sidebar/canvas ratio.", built(BuiltinType::Float), true),
				],
			),
			record_type(
				"GlorpTxn",
				"Atomic command batch.",
				vec![field(
					"commands",
					"Commands to apply atomically.",
					TypeRef::Named("CommandList".to_owned()),
					true,
				)],
			),
			record_type(
				"GlorpNotice",
				"Notice payload.",
				vec![field("message", "Notice message.", built(BuiltinType::String), true)],
			),
			list_type("StringList", "List of strings.", built(BuiltinType::String)),
			list_type(
				"CommandList",
				"List of commands.",
				TypeRef::Named("GlorpCommand".to_owned()),
			),
			list_type(
				"ConfigAssignmentList",
				"List of config assignments.",
				TypeRef::Named("ConfigAssignment".to_owned()),
			),
		],
		config: vec![
			config_field(
				"editor.preset",
				"Optional sample preset.",
				named("SamplePreset"),
				GlorpValue::String("tall".into()),
			),
			config_field(
				"editor.font",
				"Editor font choice.",
				named("FontChoice"),
				"jetbrains-mono".into(),
			),
			config_field(
				"editor.shaping",
				"Editor shaping mode.",
				named("ShapingChoice"),
				"advanced".into(),
			),
			config_field(
				"editor.wrapping",
				"Editor wrapping mode.",
				named("WrapChoice"),
				"word".into(),
			),
			config_field(
				"editor.font_size",
				"Editor font size in logical pixels.",
				built(BuiltinType::Float),
				24.0.into(),
			),
			config_field(
				"editor.line_height",
				"Editor line height in logical pixels.",
				built(BuiltinType::Float),
				32.0.into(),
			),
			config_field(
				"inspect.show_baselines",
				"Show line baselines in inspect mode.",
				built(BuiltinType::Bool),
				false.into(),
			),
			config_field(
				"inspect.show_hitboxes",
				"Show glyph hitboxes in inspect mode.",
				built(BuiltinType::Bool),
				false.into(),
			),
		],
		commands: vec![
			command(
				"glorp config set",
				"Set one config field.",
				named("ConfigAssignment"),
				named("GlorpOutcome"),
			),
			command(
				"glorp config patch",
				"Patch multiple config fields.",
				named("ConfigPatch"),
				named("GlorpOutcome"),
			),
			command(
				"glorp config reset",
				"Reset one config field to its default.",
				built(BuiltinType::String),
				named("GlorpOutcome"),
			),
			command(
				"glorp config reload",
				"Reload durable config from disk.",
				built(BuiltinType::Null),
				named("GlorpOutcome"),
			),
			command(
				"glorp config persist",
				"Persist the current config to disk.",
				built(BuiltinType::Null),
				named("GlorpOutcome"),
			),
			command(
				"glorp doc replace",
				"Replace the entire document text.",
				built(BuiltinType::String),
				named("GlorpOutcome"),
			),
			command(
				"glorp editor motion",
				"Apply a typed motion command.",
				named("EditorMotion"),
				named("GlorpOutcome"),
			),
			command(
				"glorp editor mode",
				"Apply a typed mode command.",
				named("EditorModeCommand"),
				named("GlorpOutcome"),
			),
			command(
				"glorp editor edit",
				"Apply a typed edit command.",
				named("EditorEditCommand"),
				named("GlorpOutcome"),
			),
			command(
				"glorp editor history",
				"Apply history navigation.",
				named("EditorHistoryCommand"),
				named("GlorpOutcome"),
			),
			command(
				"glorp ui sidebar select",
				"Select a sidebar tab.",
				named("SidebarTab"),
				named("GlorpOutcome"),
			),
			command(
				"glorp ui viewport scroll-to",
				"Set runtime viewport scroll position.",
				named("ScrollTarget"),
				named("GlorpOutcome"),
			),
			command(
				"glorp scene ensure",
				"Materialize scene state.",
				built(BuiltinType::Null),
				named("GlorpOutcome"),
			),
			command(
				"glorp txn",
				"Apply a transaction atomically.",
				named("GlorpTxn"),
				named("GlorpOutcome"),
			),
		],
		queries: vec![
			query(
				"glorp schema",
				"Return the runtime reflection schema.",
				named("GlorpSchema"),
			),
			query(
				"glorp get config",
				"Return the effective runtime config.",
				named("GlorpConfig"),
			),
			query(
				"glorp get state",
				"Return a snapshot of runtime state.",
				named("GlorpSnapshot"),
			),
			query(
				"glorp get document-text",
				"Return the current document text.",
				built(BuiltinType::String),
			),
			query(
				"glorp get capabilities",
				"Return stable runtime capability flags.",
				named("GlorpCapabilities"),
			),
		],
		events: vec![
			event("glorp changed", "Revisioned state change event.", named("GlorpOutcome")),
			event("glorp notice", "Runtime notices.", named("GlorpNotice")),
		],
	}
}

fn config_field(path: &str, docs: &str, ty: TypeRef, default: GlorpValue) -> ConfigFieldSchema {
	ConfigFieldSchema {
		path: path.to_owned(),
		docs: docs.to_owned(),
		ty,
		default,
		mutable: true,
	}
}

fn command(path: &str, docs: &str, input: TypeRef, output: TypeRef) -> CommandSchema {
	CommandSchema {
		path: path.to_owned(),
		docs: docs.to_owned(),
		input,
		output,
	}
}

fn query(path: &str, docs: &str, output: TypeRef) -> QuerySchema {
	QuerySchema {
		path: path.to_owned(),
		docs: docs.to_owned(),
		output,
	}
}

fn event(path: &str, docs: &str, payload: TypeRef) -> EventSchema {
	EventSchema {
		path: path.to_owned(),
		docs: docs.to_owned(),
		payload,
	}
}

fn enum_type(name: &str, docs: &str, variants: &[(&str, &str)]) -> NamedTypeSchema {
	NamedTypeSchema {
		name: name.to_owned(),
		docs: docs.to_owned(),
		kind: TypeSchema::Enum {
			variants: variants
				.iter()
				.map(|(name, docs)| EnumVariantSchema {
					name: (*name).to_owned(),
					docs: (*docs).to_owned(),
				})
				.collect(),
		},
	}
}

fn record_type(name: &str, docs: &str, fields: Vec<FieldSchema>) -> NamedTypeSchema {
	NamedTypeSchema {
		name: name.to_owned(),
		docs: docs.to_owned(),
		kind: TypeSchema::Record { fields },
	}
}

fn list_type(name: &str, docs: &str, item: TypeRef) -> NamedTypeSchema {
	NamedTypeSchema {
		name: name.to_owned(),
		docs: docs.to_owned(),
		kind: TypeSchema::List { item },
	}
}

fn field(name: &str, docs: &str, ty: TypeRef, required: bool) -> FieldSchema {
	FieldSchema {
		name: name.to_owned(),
		docs: docs.to_owned(),
		ty,
		required,
	}
}

fn built(kind: BuiltinType) -> TypeRef {
	TypeRef::Builtin(kind)
}

fn named(name: &str) -> TypeRef {
	TypeRef::Named(name.to_owned())
}
