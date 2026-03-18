use crate::{
	CanvasTarget, EditorMode, FontChoice, GlorpConfig, GlorpRevisions, InspectConfig, LayoutRectView, SamplePreset,
	ShapingChoice, SidebarTab, WrapChoice,
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(tag = "op", content = "input", rename_all = "kebab-case")]
pub enum GlorpQuery {
	Schema,
	Config,
	Snapshot(SnapshotQuery),
	DocumentText,
	Selection,
	InspectDetails(InspectDetailsQuery),
	Perf,
	Ui,
	Capabilities,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(tag = "kind", content = "output", rename_all = "kebab-case")]
pub enum GlorpQueryResult {
	Schema(crate::GlorpSchema),
	Config(GlorpConfig),
	Snapshot(GlorpSnapshot),
	DocumentText(String),
	Selection(SelectionStateView),
	InspectDetails(InspectDetailsView),
	Perf(PerfDashboardView),
	Ui(UiStateView),
	Capabilities(GlorpCapabilities),
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct SnapshotQuery {
	pub scene: SceneLevel,
	pub include_document_text: bool,
}

#[derive(Debug, Clone, Copy, Default, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct InspectDetailsQuery {
	pub target: Option<CanvasTarget>,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct InspectStateView {
	pub hovered_target: Option<CanvasTarget>,
	pub selected_target: Option<CanvasTarget>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct SelectionStateView {
	pub mode: EditorMode,
	pub range: Option<crate::TextRange>,
	pub selected_text: Option<String>,
	pub selection_head: Option<u64>,
	pub pointer_anchor: Option<u64>,
	pub viewport_target: Option<LayoutRectView>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct InspectDetailsView {
	pub hovered_target: Option<CanvasTarget>,
	pub selected_target: Option<CanvasTarget>,
	pub active_target: Option<CanvasTarget>,
	pub warnings: Vec<String>,
	pub interaction_details: String,
	pub scene: Option<InspectSceneView>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct InspectSceneView {
	pub revision: u64,
	pub run_count: usize,
	pub cluster_count: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct PerfStateView {
	pub scene_builds: usize,
	pub scene_build_millis: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct PerfDashboardView {
	pub overview: PerfOverviewView,
	pub metrics: Vec<PerfMetricSummaryView>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct PerfOverviewView {
	pub editor_mode: EditorMode,
	pub editor_bytes: usize,
	pub text_lines: usize,
	pub layout_width: f32,
	pub scene_ready: bool,
	pub scene_revision: Option<u64>,
	pub scene_width: f32,
	pub scene_height: f32,
	pub run_count: usize,
	pub cluster_count: usize,
	pub warning_count: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct PerfMetricSummaryView {
	pub label: String,
	pub total_samples: u64,
	pub total_millis: f64,
	pub last_millis: f64,
	pub avg_millis: f64,
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct GlorpSessionView {
	pub socket: String,
	pub repo_root: Option<String>,
	pub capabilities: GlorpCapabilities,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct GlorpEventStreamView {
	pub token: u64,
	pub subscription: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct ExampleView {
	pub font: FontChoice,
	pub shaping: ShapingChoice,
	pub inspect: InspectConfig,
	pub preset: Option<SamplePreset>,
}
