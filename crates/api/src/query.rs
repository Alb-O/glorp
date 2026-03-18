use crate::{EditorMode, GlorpRevisions, LayoutRectView, WrapChoice};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct GlorpCapabilities {
	pub transactions: bool,
	pub subscriptions: bool,
	pub transports: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct EditorStateView {
	pub revisions: GlorpRevisions,
	pub mode: EditorMode,
	pub selection: Option<crate::TextRange>,
	pub selected_text: Option<String>,
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
