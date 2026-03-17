use {
	crate::{
		editor::{EditorEngine, EditorIntent, EditorMode, EditorOutcome, EditorViewportMetrics},
		overlay::LayoutRect,
		presentation::{EditorPresentation, ScenePresentation, SessionSnapshot},
		scene::{SceneConfig, make_font_system},
	},
	cosmic_text::FontSystem,
	std::time::{Duration, Instant},
};

#[cfg(test)]
use crate::editor::EditorViewState;

/// Owns the editable document plus the coherent render snapshot consumed by the
/// rest of the app.
pub(super) struct DocumentSession {
	editor: EditorEngine,
	snapshot: SessionSnapshot,
	next_scene_revision: u64,
	font_system: FontSystem,
	#[cfg(test)]
	derived_scene_build_count: usize,
}

impl DocumentSession {
	pub(super) fn new(text: &str, config: SceneConfig) -> Self {
		let mut font_system = make_font_system();
		let editor = EditorEngine::new(&mut font_system, text, config);
		let snapshot = SessionSnapshot::new(build_editor_presentation(&editor, 1));

		Self {
			editor,
			snapshot,
			next_scene_revision: 0,
			font_system,
			#[cfg(test)]
			derived_scene_build_count: 0,
		}
	}

	pub(super) fn snapshot(&self) -> &SessionSnapshot {
		&self.snapshot
	}

	pub(super) fn text(&self) -> &str {
		self.editor.text()
	}

	pub(super) fn mode(&self) -> EditorMode {
		self.snapshot.mode()
	}

	#[cfg(test)]
	pub(super) fn view_state(&self) -> EditorViewState {
		self.snapshot.editor.editor.clone()
	}

	pub(super) fn viewport_metrics(&self) -> EditorViewportMetrics {
		self.snapshot.editor.viewport_metrics
	}

	pub(super) fn viewport_target(&self) -> Option<LayoutRect> {
		self.snapshot.editor.editor.viewport_target
	}

	#[cfg(test)]
	pub(super) fn history_depths(&self) -> (usize, usize) {
		(self.snapshot.editor.undo_depth, self.snapshot.editor.redo_depth)
	}

	pub(super) fn sync_width(&mut self, width: f32) -> Option<Duration> {
		let started = Instant::now();
		self.editor.sync_buffer_width(&mut self.font_system, width).then(|| {
			self.refresh_editor_snapshot();
			self.invalidate_scene();
			started.elapsed()
		})
	}

	pub(super) fn sync_config(&mut self, config: SceneConfig) -> bool {
		let changed = self.editor.sync_buffer_config(&mut self.font_system, config);
		if changed {
			self.refresh_editor_snapshot();
			self.invalidate_scene();
		}
		changed
	}

	pub(super) fn replace_document(&mut self, text: &str, config: SceneConfig) {
		self.editor.reset(&mut self.font_system, text, config);
		self.refresh_editor_snapshot();
		self.invalidate_scene();
	}

	pub(super) fn apply_editor_intent(&mut self, intent: EditorIntent) -> EditorOutcome {
		let outcome = self.editor.apply(&mut self.font_system, intent);

		if outcome.changed() {
			self.refresh_editor_snapshot();
		}

		if outcome.document_changed() {
			self.invalidate_scene();
		}

		outcome
	}

	pub(super) fn ensure_scene(&mut self) -> Option<Duration> {
		if self.snapshot.scene.is_some() {
			return None;
		}

		let revision = self.next_scene_revision + 1;
		let started = Instant::now();
		self.snapshot.scene = Some(build_scene_presentation(&self.editor, revision));
		self.next_scene_revision = revision;
		#[cfg(test)]
		{
			self.derived_scene_build_count += 1;
		}
		Some(started.elapsed())
	}

	#[cfg(test)]
	pub(super) fn derived_scene_build_count(&self) -> usize {
		self.derived_scene_build_count
	}

	fn refresh_editor_snapshot(&mut self) {
		let revision = self.snapshot.editor.revision + 1;
		self.snapshot.editor = build_editor_presentation(&self.editor, revision);
	}

	fn invalidate_scene(&mut self) {
		self.snapshot.scene = None;
	}
}

fn build_editor_presentation(editor: &EditorEngine, revision: u64) -> EditorPresentation {
	let text_layer = editor.text_layer_state();
	let viewport_metrics = editor.viewport_metrics();
	let editor_view = editor.view_state();
	let (undo_depth, redo_depth) = editor.history_depths();

	EditorPresentation::new(
		revision,
		viewport_metrics,
		text_layer,
		editor_view,
		editor.text().len(),
		undo_depth,
		redo_depth,
	)
}

fn build_scene_presentation(editor: &EditorEngine, revision: u64) -> ScenePresentation {
	let layout = editor.shared_document_layout();
	ScenePresentation::new(revision, layout)
}
