use cosmic_text::FontSystem;

use crate::editor::{
	EditorEngine, EditorIntent, EditorMode, EditorOutcome, EditorTextLayerState, EditorViewState, EditorViewportMetrics,
};
use crate::overlay::OverlayPrimitive;
use crate::scene::{LayoutScene, LayoutSceneModel, SceneConfig, make_font_system};
use crate::types::CanvasTarget;

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

	pub(super) fn viewport_metrics(&self) -> EditorViewportMetrics {
		self.editor.viewport_metrics()
	}

	pub(super) fn text_layer_state(&self) -> EditorTextLayerState {
		self.editor.text_layer_state()
	}

	pub(super) fn history_depths(&self) -> (usize, usize) {
		self.editor.history_depths()
	}

	pub(super) fn selection_details(&self) -> String {
		self.editor.selection_details()
	}

	pub(super) fn inspect_overlay_primitives(
		&self, hovered_target: Option<CanvasTarget>, selected_target: Option<CanvasTarget>, layout_width: f32,
		show_hitboxes: bool,
	) -> std::sync::Arc<[OverlayPrimitive]> {
		self.scene
			.scene()
			.inspect_overlay_primitives(hovered_target, selected_target, layout_width, show_hitboxes)
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
