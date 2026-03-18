use crate::{
	CanvasTarget, EditorMode, FontChoice, GlorpConfig, GlorpRevisions, InspectConfig, LayoutRectView, SamplePreset,
	ShapingChoice, SidebarTab, WrapChoice,
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum GlorpQuery {
	Schema,
	Config,
	Snapshot {
		scene: SceneLevel,
		include_document_text: bool,
	},
	DocumentText,
	Capabilities,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum GlorpQueryResult {
	Schema(crate::GlorpSchema),
	Config(GlorpConfig),
	Snapshot(GlorpSnapshot),
	DocumentText(String),
	Capabilities(GlorpCapabilities),
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum SceneLevel {
	Omit,
	IfReady,
	Materialize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct GlorpCapabilities {
	pub transactions: bool,
	pub subscriptions: bool,
	pub transports: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct GlorpSnapshot {
	pub revisions: GlorpRevisions,
	pub config: GlorpConfig,
	pub editor: EditorStateView,
	pub scene: Option<SceneStateView>,
	pub inspect: InspectStateView,
	pub perf: PerfStateView,
	pub ui: UiStateView,
	pub document_text: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct EditorStateView {
	pub mode: EditorMode,
	pub selection: Option<crate::TextRange>,
	pub selection_head: Option<u64>,
	pub pointer_anchor: Option<u64>,
	pub text_bytes: usize,
	pub text_lines: usize,
	pub undo_depth: usize,
	pub redo_depth: usize,
	pub viewport: EditorViewportView,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct EditorViewportView {
	pub wrapping: WrapChoice,
	pub measured_width: f32,
	pub measured_height: f32,
	pub viewport_target: Option<LayoutRectView>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct SceneStateView {
	pub revision: u64,
	pub measured_width: f32,
	pub measured_height: f32,
	pub run_count: usize,
	pub cluster_count: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct InspectStateView {
	pub hovered_target: Option<CanvasTarget>,
	pub selected_target: Option<CanvasTarget>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct PerfStateView {
	pub scene_builds: usize,
	pub scene_build_millis: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct UiStateView {
	pub active_tab: SidebarTab,
	pub canvas_focused: bool,
	pub canvas_scroll_x: f32,
	pub canvas_scroll_y: f32,
	pub layout_width: f32,
	pub viewport_width: f32,
	pub viewport_height: f32,
	pub pane_ratio: f32,
}

#[allow(dead_code)]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct ExampleView {
	pub font: FontChoice,
	pub shaping: ShapingChoice,
	pub inspect: InspectConfig,
	pub preset: Option<SamplePreset>,
}
