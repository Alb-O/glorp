use glorp_api::{EditorContextView, EditorHistoryCommand, GlorpConfig, GlorpOutcome, GlorpRevisions};

pub const LARGE_PAYLOAD_BYTES: usize = 4096;

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
	pub document_sync: Option<GuiDocumentSyncRef>,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct GuiSceneSummary {
	pub revision: u64,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum GuiDocumentSyncReason {
	Bootstrap,
	LargeEdit,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct GuiDocumentSyncRef {
	pub revision: u64,
	pub bytes: usize,
	pub reason: GuiDocumentSyncReason,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct GuiDocumentFetchRequest {
	pub revision: u64,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct GuiDocumentFetchResponse {
	pub revision: u64,
	pub bytes: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct GuiSharedDelta {
	pub outcome: GlorpOutcome,
	pub undo_depth: usize,
	pub redo_depth: usize,
	pub config: Option<GlorpConfig>,
	pub scene_summary: GuiSceneSummary,
	pub document_sync: Option<GuiDocumentSyncRef>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum GuiSessionRequest {
	Call(glorp_api::GlorpCall),
	Edit(GuiEditRequest),
	GuiFrame(GuiLayoutRequest),
	DocumentFetch(GuiDocumentFetchRequest),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum GuiSessionResponse {
	Call(Result<glorp_api::GlorpCallResult, glorp_api::GlorpError>),
	Edit(Result<GuiEditResponse, glorp_api::GlorpError>),
	GuiFrame(Result<GuiRuntimeFrame, glorp_api::GlorpError>),
	DocumentFetch(Result<GuiDocumentFetchResponse, glorp_api::GlorpError>),
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
	pub document_text: Option<String>,
	pub undo_depth: usize,
	pub redo_depth: usize,
	pub scene_summary: GuiSceneSummary,
	pub document_sync: Option<GuiDocumentSyncRef>,
}
