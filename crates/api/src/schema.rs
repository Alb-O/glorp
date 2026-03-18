use crate::{ConfigPath, EnumValue, GlorpValue};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct GlorpSchema {
	pub version: u32,
	pub named_types: Vec<NamedTypeSchema>,
	pub config: Vec<ConfigFieldSchema>,
	pub commands: Vec<CommandSchema>,
	pub queries: Vec<QuerySchema>,
	pub helpers: Vec<HelperSchema>,
	pub events: Vec<EventSchema>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct CommandSchema {
	pub path: String,
	pub docs: String,
	pub input: TypeRef,
	pub output: TypeRef,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct QuerySchema {
	pub path: String,
	pub docs: String,
	pub input: Option<TypeRef>,
	pub output: TypeRef,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct HelperSchema {
	pub path: String,
	pub docs: String,
	pub kind: HelperKind,
	pub input: Option<TypeRef>,
	pub output: TypeRef,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct EventSchema {
	pub path: String,
	pub docs: String,
	pub payload: TypeRef,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum HelperKind {
	ConfigValidate,
	SessionAttach,
	SessionShutdown,
	EventsSubscribe,
	EventsNext,
	EventsUnsubscribe,
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
	Any,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum TypeSchema {
	Enum { variants: Vec<EnumVariantSchema> },
	Record { fields: Vec<FieldSchema> },
	List { item: TypeRef },
	Option { item: TypeRef },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct FieldSchema {
	pub name: String,
	pub docs: String,
	pub ty: TypeRef,
	pub required: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct EnumVariantSchema {
	pub name: String,
	pub docs: String,
}

#[must_use]
pub fn glorp_schema() -> GlorpSchema {
	GlorpSchema {
		version: 2,
		named_types: vec![
			enum_type_from::<crate::SamplePreset>("SamplePreset", "Built-in sample document presets."),
			enum_type_from::<crate::WrapChoice>("WrapChoice", "Stable editor wrapping choices."),
			enum_type_from::<crate::FontChoice>("FontChoice", "Stable editor font families."),
			enum_type_from::<crate::ShapingChoice>("ShapingChoice", "Stable shaping choices."),
			enum_type_from::<crate::SidebarTab>("SidebarTab", "Stable sidebar tabs."),
			enum_type_from::<crate::EditorMotion>("EditorMotion", "Typed editor motions."),
			enum_type_from::<crate::EditorModeCommand>("EditorModeCommand", "Typed mode transitions."),
			enum_type_from::<crate::EditorHistoryCommand>("EditorHistoryCommand", "Typed undo/redo operations."),
			enum_type_from::<crate::EditorMode>("EditorMode", "Stable editor modes."),
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
					field("value", "Config value.", built(BuiltinType::Any), true),
				],
			),
			record_type(
				"ConfigPatch",
				"Nested config patch input.",
				vec![field(
					"patch",
					"Nested config patch record.",
					built(BuiltinType::Any),
					true,
				)],
			),
			record_type(
				"ConfigPathInput",
				"One config path input.",
				vec![field("path", "Config path.", built(BuiltinType::String), true)],
			),
			record_type(
				"TextInput",
				"One text input.",
				vec![field("text", "Text input.", built(BuiltinType::String), true)],
			),
			record_type(
				"EditorMotionInput",
				"One editor motion input.",
				vec![field(
					"motion",
					"Editor motion name.",
					TypeRef::Named("EditorMotion".to_owned()),
					true,
				)],
			),
			record_type(
				"EditorModeInput",
				"One editor mode input.",
				vec![field(
					"mode",
					"Editor mode command.",
					TypeRef::Named("EditorModeCommand".to_owned()),
					true,
				)],
			),
			record_type(
				"EditorHistoryInput",
				"One editor history input.",
				vec![field(
					"action",
					"Editor history action.",
					TypeRef::Named("EditorHistoryCommand".to_owned()),
					true,
				)],
			),
			record_type(
				"SidebarTabInput",
				"One sidebar tab input.",
				vec![field(
					"tab",
					"Sidebar tab.",
					TypeRef::Named("SidebarTab".to_owned()),
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
				"PaneRatioInput",
				"Sidebar/canvas ratio input.",
				vec![field("ratio", "Sidebar/canvas ratio.", built(BuiltinType::Float), true)],
			),
			record_type(
				"InspectDetailsInput",
				"Inspect-details query input.",
				vec![field(
					"target",
					"Optional canvas target run:<n> or cluster:<n>.",
					built(BuiltinType::String),
					false,
				)],
			),
			record_type(
				"StreamTokenInput",
				"Subscription token input.",
				vec![field("token", "Subscription token.", built(BuiltinType::Int), true)],
			),
			list_type(
				"GlorpCommandList",
				"List of typed public commands.",
				built(BuiltinType::Any),
			),
			record_type(
				"GlorpTxn",
				"Multiple typed public commands applied atomically.",
				vec![field(
					"commands",
					"Ordered typed commands.",
					TypeRef::Named("GlorpCommandList".to_owned()),
					true,
				)],
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
					field(
						"inspect",
						"Inspect state view.",
						TypeRef::Named("InspectStateView".to_owned()),
						true,
					),
					field(
						"perf",
						"Performance state view.",
						TypeRef::Named("PerfStateView".to_owned()),
						true,
					),
					field("ui", "UI state view.", TypeRef::Named("UiStateView".to_owned()), true),
					field(
						"document_text",
						"Document text when requested.",
						built(BuiltinType::String),
						false,
					),
				],
			),
			record_type(
				"EditorStateView",
				"Stable editor state view.",
				vec![
					field("mode", "Editor mode.", TypeRef::Named("EditorMode".to_owned()), true),
					field(
						"selection",
						"Current selection range.",
						TypeRef::Named("TextRange".to_owned()),
						false,
					),
					field(
						"selection_head",
						"Selection head byte offset.",
						built(BuiltinType::Int),
						false,
					),
					field(
						"pointer_anchor",
						"Pointer anchor byte offset.",
						built(BuiltinType::Int),
						false,
					),
					field("text_bytes", "Document size in bytes.", built(BuiltinType::Int), true),
					field("text_lines", "Document line count.", built(BuiltinType::Int), true),
					field("undo_depth", "Undo depth.", built(BuiltinType::Int), true),
					field("redo_depth", "Redo depth.", built(BuiltinType::Int), true),
					field(
						"viewport",
						"Viewport-facing editor metrics.",
						TypeRef::Named("EditorViewportView".to_owned()),
						true,
					),
				],
			),
			record_type(
				"EditorViewportView",
				"Viewport-facing editor metrics.",
				vec![
					field(
						"wrapping",
						"Current wrapping mode.",
						TypeRef::Named("WrapChoice".to_owned()),
						true,
					),
					field(
						"measured_width",
						"Measured content width.",
						built(BuiltinType::Float),
						true,
					),
					field(
						"measured_height",
						"Measured content height.",
						built(BuiltinType::Float),
						true,
					),
					field(
						"viewport_target",
						"Current viewport reveal target.",
						TypeRef::Named("LayoutRectView".to_owned()),
						false,
					),
				],
			),
			record_type(
				"SceneStateView",
				"Stable scene state view.",
				vec![
					field("revision", "Scene revision.", built(BuiltinType::Int), true),
					field("measured_width", "Measured width.", built(BuiltinType::Float), true),
					field("measured_height", "Measured height.", built(BuiltinType::Float), true),
					field("run_count", "Layout run count.", built(BuiltinType::Int), true),
					field("cluster_count", "Layout cluster count.", built(BuiltinType::Int), true),
				],
			),
			record_type(
				"InspectStateView",
				"Stable inspect state view.",
				vec![
					field(
						"hovered_target",
						"Hovered inspect target.",
						TypeRef::Named("CanvasTarget".to_owned()),
						false,
					),
					field(
						"selected_target",
						"Selected inspect target.",
						TypeRef::Named("CanvasTarget".to_owned()),
						false,
					),
				],
			),
			record_type(
				"PerfStateView",
				"Stable runtime perf counters.",
				vec![
					field("scene_builds", "Scene build count.", built(BuiltinType::Int), true),
					field(
						"scene_build_millis",
						"Accumulated scene build millis.",
						built(BuiltinType::Float),
						true,
					),
				],
			),
			record_type(
				"SelectionStateView",
				"Focused selection read model.",
				vec![
					field("mode", "Editor mode.", TypeRef::Named("EditorMode".to_owned()), true),
					field(
						"range",
						"Current selection range.",
						TypeRef::Named("TextRange".to_owned()),
						false,
					),
					field(
						"selected_text",
						"Selected text if any.",
						built(BuiltinType::String),
						false,
					),
					field(
						"selection_head",
						"Selection head byte offset.",
						built(BuiltinType::Int),
						false,
					),
					field(
						"pointer_anchor",
						"Pointer anchor byte offset.",
						built(BuiltinType::Int),
						false,
					),
					field(
						"viewport_target",
						"Current viewport reveal target.",
						TypeRef::Named("LayoutRectView".to_owned()),
						false,
					),
				],
			),
			record_type(
				"InspectDetailsView",
				"Rich inspect read model.",
				vec![
					field(
						"hovered_target",
						"Hovered inspect target.",
						TypeRef::Named("CanvasTarget".to_owned()),
						false,
					),
					field(
						"selected_target",
						"Selected inspect target.",
						TypeRef::Named("CanvasTarget".to_owned()),
						false,
					),
					field(
						"active_target",
						"Active inspect target.",
						TypeRef::Named("CanvasTarget".to_owned()),
						false,
					),
					field(
						"warnings",
						"Scene warnings.",
						TypeRef::Named("StringList".to_owned()),
						true,
					),
					field(
						"interaction_details",
						"Human-readable target details.",
						built(BuiltinType::String),
						true,
					),
					field(
						"scene",
						"Inspect scene summary.",
						TypeRef::Named("InspectSceneView".to_owned()),
						false,
					),
				],
			),
			record_type(
				"InspectSceneView",
				"Inspect-side scene summary.",
				vec![
					field("revision", "Scene revision.", built(BuiltinType::Int), true),
					field("run_count", "Layout run count.", built(BuiltinType::Int), true),
					field("cluster_count", "Layout cluster count.", built(BuiltinType::Int), true),
				],
			),
			record_type(
				"PerfDashboardView",
				"Rich runtime perf dashboard.",
				vec![
					field(
						"overview",
						"Perf overview.",
						TypeRef::Named("PerfOverviewView".to_owned()),
						true,
					),
					field(
						"metrics",
						"Perf metric summaries.",
						TypeRef::Named("PerfMetricSummaryList".to_owned()),
						true,
					),
				],
			),
			record_type(
				"PerfOverviewView",
				"Perf overview summary.",
				vec![
					field(
						"editor_mode",
						"Editor mode.",
						TypeRef::Named("EditorMode".to_owned()),
						true,
					),
					field("editor_bytes", "Document size in bytes.", built(BuiltinType::Int), true),
					field("text_lines", "Document line count.", built(BuiltinType::Int), true),
					field("layout_width", "Current layout width.", built(BuiltinType::Float), true),
					field(
						"scene_ready",
						"Whether scene data is materialized.",
						built(BuiltinType::Bool),
						true,
					),
					field(
						"scene_revision",
						"Current scene revision.",
						built(BuiltinType::Int),
						false,
					),
					field("scene_width", "Scene width.", built(BuiltinType::Float), true),
					field("scene_height", "Scene height.", built(BuiltinType::Float), true),
					field("run_count", "Layout run count.", built(BuiltinType::Int), true),
					field("cluster_count", "Layout cluster count.", built(BuiltinType::Int), true),
					field("warning_count", "Scene warning count.", built(BuiltinType::Int), true),
				],
			),
			record_type(
				"PerfMetricSummaryView",
				"Perf metric summary row.",
				vec![
					field("label", "Metric label.", built(BuiltinType::String), true),
					field("total_samples", "Total samples.", built(BuiltinType::Int), true),
					field("total_millis", "Total millis.", built(BuiltinType::Float), true),
					field("last_millis", "Most recent millis.", built(BuiltinType::Float), true),
					field("avg_millis", "Average millis.", built(BuiltinType::Float), true),
				],
			),
			record_type(
				"TextRange",
				"Byte range in the document.",
				vec![
					field("start", "Start byte offset.", built(BuiltinType::Int), true),
					field("end", "End byte offset.", built(BuiltinType::Int), true),
				],
			),
			record_type(
				"LayoutRectView",
				"Rectangle in layout coordinates.",
				vec![
					field("x", "Left position.", built(BuiltinType::Float), true),
					field("y", "Top position.", built(BuiltinType::Float), true),
					field("width", "Rectangle width.", built(BuiltinType::Float), true),
					field("height", "Rectangle height.", built(BuiltinType::Float), true),
				],
			),
			record_type(
				"GlorpEventStreamView",
				"Subscription handle for event polling.",
				vec![
					field("token", "Subscription token.", built(BuiltinType::Int), true),
					field("subscription", "Subscription name.", built(BuiltinType::String), true),
				],
			),
			record_type(
				"OkView",
				"Boolean acknowledgement.",
				vec![field("ok", "Acknowledgement flag.", built(BuiltinType::Bool), true)],
			),
			record_type(
				"TokenAckView",
				"Acknowledgement with a token payload.",
				vec![
					field("ok", "Acknowledgement flag.", built(BuiltinType::Bool), true),
					field("token", "Subscription token.", built(BuiltinType::Int), true),
				],
			),
			record_type(
				"GlorpSessionView",
				"Resolved live session endpoint.",
				vec![
					field("socket", "Socket path.", built(BuiltinType::String), true),
					field("repo_root", "Resolved repo root.", built(BuiltinType::String), false),
					field(
						"capabilities",
						"Runtime capabilities.",
						TypeRef::Named("GlorpCapabilities".to_owned()),
						true,
					),
				],
			),
			record_type(
				"CanvasTarget",
				"Canvas inspect target.",
				vec![
					field("run", "Run index when targeting a run.", built(BuiltinType::Int), false),
					field(
						"cluster",
						"Cluster index when targeting a cluster.",
						built(BuiltinType::Int),
						false,
					),
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
						"canvas_focused",
						"Whether the canvas owns focus.",
						built(BuiltinType::Bool),
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
					field("layout_width", "Current layout width.", built(BuiltinType::Float), true),
					field("viewport_width", "Viewport width.", built(BuiltinType::Float), true),
					field("viewport_height", "Viewport height.", built(BuiltinType::Float), true),
					field("pane_ratio", "Sidebar/canvas ratio.", built(BuiltinType::Float), true),
				],
			),
			record_type(
				"GlorpNotice",
				"Notice payload.",
				vec![field("message", "Notice message.", built(BuiltinType::String), true)],
			),
			list_type("StringList", "List of strings.", built(BuiltinType::String)),
			list_type(
				"PerfMetricSummaryList",
				"List of perf metric summaries.",
				TypeRef::Named("PerfMetricSummaryView".to_owned()),
			),
		],
		config: crate::config_schema_fields(),
		commands: crate::command_schemas(),
		queries: crate::query_schemas(),
		helpers: crate::helper_schemas(),
		events: vec![
			event("glorp changed", "Revisioned state change event.", named("GlorpOutcome")),
			event("glorp notice", "Runtime notices.", named("GlorpNotice")),
		],
	}
}

fn enum_type_from<T>(name: &str, docs: &str) -> NamedTypeSchema
where
	T: EnumValue, {
	enum_type(
		name,
		docs,
		&T::allowed_values()
			.iter()
			.map(|value| (*value, T::docs(value).unwrap_or_default()))
			.collect::<Vec<_>>(),
	)
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
				.map(|&(name, docs)| EnumVariantSchema {
					name: name.to_owned(),
					docs: docs.to_owned(),
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

const fn built(kind: BuiltinType) -> TypeRef {
	TypeRef::Builtin(kind)
}

fn named(name: &str) -> TypeRef {
	TypeRef::Named(name.to_owned())
}
