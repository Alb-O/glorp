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
pub enum GuiCommand {
	SceneEnsure,
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
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GuiRuntimeFrame {
	pub config: GlorpConfig,
	pub layout_width: f32,
	pub revisions: GlorpRevisions,
	pub document_text: String,
	pub undo_depth: usize,
	pub redo_depth: usize,
	pub scene: Option<ScenePresentation>,
}
