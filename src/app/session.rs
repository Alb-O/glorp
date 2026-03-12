use cosmic_text::FontSystem;

use crate::editor::{EditorEngine, EditorIntent, EditorMode, EditorOutcome, EditorViewState};
use crate::scene::{LayoutScene, LayoutSceneModel, SceneConfig, make_font_system};

pub(super) struct SceneSession {
	editor: EditorEngine,
	scene: LayoutSceneModel,
	font_system: FontSystem,
}

impl SceneSession {
	pub(super) fn new(text: &str, config: SceneConfig) -> Self {
		let mut font_system = make_font_system();
		let editor = EditorEngine::new(&mut font_system, text, config);
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

	pub(super) fn apply_editor_intent(&mut self, intent: EditorIntent) -> EditorOutcome {
		self.editor.apply(&mut self.font_system, intent)
	}

	pub(super) fn rebuild(&mut self, config: SceneConfig) {
		self.editor.sync_buffer_config(&mut self.font_system, config);
		self.scene
			.rebuild(&mut self.font_system, self.editor.text(), self.editor.buffer(), config);
	}
}
