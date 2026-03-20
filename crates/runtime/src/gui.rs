use {
	glorp_api::{EditorContextView, EditorHistoryCommand, GlorpConfig, GlorpOutcome, GlorpRevisions},
	glorp_editor::ScenePresentation,
};

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash)]
pub enum SidebarTab {
	Controls,
	Inspect,
	Perf,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum GuiEditCommand {
	InsertText(String),
	Backspace,
	DeleteForward,
	DeleteSelection,
	History(EditorHistoryCommand),
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct GuiLayoutRequest {
	pub layout_width: f32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct GuiEditRequest {
	pub layout: GuiLayoutRequest,
	pub context: EditorContextView,
	pub command: GuiEditCommand,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct GuiEditResponse {
	pub outcome: GlorpOutcome,
	pub next_context: EditorContextView,
	pub revisions: GlorpRevisions,
	pub undo_depth: usize,
	pub redo_depth: usize,
	pub scene_summary: GuiSceneSummary,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct GuiSceneSummary {
	pub revision: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct GuiSharedDelta {
	pub outcome: GlorpOutcome,
	pub undo_depth: usize,
	pub redo_depth: usize,
	pub config: Option<GlorpConfig>,
	pub scene_summary: GuiSceneSummary,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum GuiSessionRequest {
	Call(glorp_api::GlorpCall),
	Edit(GuiEditRequest),
	GuiFrame(GuiLayoutRequest),
	SceneFetch(GuiLayoutRequest),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum GuiSessionResponse {
	Call(Result<glorp_api::GlorpCallResult, glorp_api::GlorpError>),
	Edit(Result<GuiEditResponse, glorp_api::GlorpError>),
	GuiFrame(Result<GuiRuntimeFrame, glorp_api::GlorpError>),
	SceneFetch(Result<ScenePresentation, glorp_api::GlorpError>),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum GuiSessionClientMessage {
	Request { id: u64, body: GuiSessionRequest },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum GuiSessionHostMessage {
	Ready { frame: Box<GuiRuntimeFrame> },
	Reply { id: u64, body: GuiSessionResponse },
	Changed(GuiSharedDelta),
	Closed,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GuiRuntimeFrame {
	pub config: GlorpConfig,
	pub layout_width: f32,
	pub revisions: GlorpRevisions,
	pub document_text: String,
	pub undo_depth: usize,
	pub redo_depth: usize,
	pub scene_summary: GuiSceneSummary,
}
