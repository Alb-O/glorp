use {
	crate::state::UiRuntimeState,
	glorp_api::{GlorpConfig, GlorpRevisions},
	glorp_editor::SessionSnapshot,
};

#[derive(Debug, Clone)]
pub struct GuiRuntimeFrame {
	pub config: GlorpConfig,
	pub ui: UiRuntimeState,
	pub revisions: GlorpRevisions,
	pub snapshot: SessionSnapshot,
	pub document_text: String,
}
