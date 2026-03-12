use cosmic_text::FontSystem;

use crate::editor::{EditorBuffer, EditorCommand, EditorMode, EditorViewState};
use crate::scene::{LayoutScene, LayoutSceneModel, SceneConfig, make_font_system};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct SessionUpdate {
	pub(super) document_changed: bool,
	pub(super) view_changed: bool,
	pub(super) scene_needs_rebuild: bool,
}

pub(super) struct SceneSession {
	editor: EditorBuffer,
	scene: LayoutSceneModel,
	font_system: FontSystem,
}

impl SceneSession {
	pub(super) fn new(text: &str, config: SceneConfig) -> Self {
		let mut font_system = make_font_system();
		let editor = EditorBuffer::new(&mut font_system, text, config);
		let scene = LayoutSceneModel::new(&mut font_system, editor.text(), editor.buffer(), config);

		Self {
			editor,
			scene,
			font_system,
		}
	}

	pub(super) fn scene(&self) -> &LayoutScene {
		self.scene.scene()
	}

	pub(super) fn text(&self) -> &str {
		self.editor.text()
	}

	pub(super) fn mode(&self) -> EditorMode {
		self.editor.mode()
	}

	pub(super) fn view_state(&self) -> EditorViewState {
		self.editor.view_state()
	}

	pub(super) fn history_depths(&self) -> (usize, usize) {
		self.editor.history_depths()
	}

	pub(super) fn selection_details(&self) -> String {
		self.editor.selection_details()
	}

	pub(super) fn sync_width(&mut self, width: f32) {
		self.editor.sync_buffer_width(&mut self.font_system, width);
	}

	pub(super) fn reset_with_preset(&mut self, text: &str, config: SceneConfig) {
		self.editor.reset(&mut self.font_system, text, config);
		self.scene
			.rebuild(&mut self.font_system, self.editor.text(), self.editor.buffer(), config);
	}

	pub(super) fn apply_editor_command(&mut self, command: EditorCommand) -> SessionUpdate {
		let update = self.editor.apply(&mut self.font_system, command);

		SessionUpdate {
			document_changed: update.document_changed(),
			view_changed: update.view_changed(),
			scene_needs_rebuild: update.document_changed(),
		}
	}

	pub(super) fn rebuild(&mut self, config: SceneConfig) {
		self.editor.sync_buffer_config(&mut self.font_system, config);
		self.scene
			.rebuild(&mut self.font_system, self.editor.text(), self.editor.buffer(), config);
	}
}
