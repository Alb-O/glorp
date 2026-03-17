use {
	crate::{
		editor::{EditorEngine, EditorIntent, EditorMode, EditorOutcome, EditorViewportMetrics},
		overlay::LayoutRect,
		presentation::{DerivedScenePresentation, EditorPresentation},
		scene::{DocumentLayout, SceneConfig, make_font_system},
	},
	cosmic_text::FontSystem,
	std::time::{Duration, Instant},
};

#[cfg(test)]
use crate::editor::EditorViewState;

/// App-facing summary of what changed after applying an editor intent.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct DocumentChanges {
	bits: u8,
}

impl DocumentChanges {
	const DOCUMENT_CHANGED: u8 = 1 << 0;
	const VIEW_CHANGED: u8 = 1 << 1;
	const SELECTION_CHANGED: u8 = 1 << 2;
	const MODE_CHANGED: u8 = 1 << 3;

	const fn with(self, flag: u8, enabled: bool) -> Self {
		// Keep the app-facing change summary compact without re-introducing a
		// wider enum matrix for mostly-independent editor deltas.
		Self {
			bits: self.bits | if enabled { flag } else { 0 },
		}
	}

	pub(crate) const fn changed(self) -> bool {
		self.bits != 0
	}

	pub(crate) const fn document_changed(self) -> bool {
		self.bits & Self::DOCUMENT_CHANGED != 0
	}

	pub(crate) const fn view_changed(self) -> bool {
		self.bits & Self::VIEW_CHANGED != 0
	}

	pub(crate) const fn selection_changed(self) -> bool {
		self.bits & Self::SELECTION_CHANGED != 0
	}

	pub(crate) const fn mode_changed(self) -> bool {
		self.bits & Self::MODE_CHANGED != 0
	}
}

/// App-facing summary of what changed after applying an editor intent.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct DocumentUpdate {
	/// The compact change set for this update.
	pub(crate) changes: DocumentChanges,
	/// The latest viewport reveal target for the updated editor state.
	pub(crate) viewport_target: Option<LayoutRect>,
}

impl DocumentUpdate {
	/// Returns whether any app-visible editor state changed.
	pub(crate) fn changed(&self) -> bool {
		self.changes.changed()
	}

	pub(crate) fn document_changed(&self) -> bool {
		self.changes.document_changed()
	}

	pub(crate) fn view_changed(&self) -> bool {
		self.changes.view_changed()
	}

	pub(crate) fn selection_changed(&self) -> bool {
		self.changes.selection_changed()
	}

	pub(crate) fn mode_changed(&self) -> bool {
		self.changes.mode_changed()
	}
}

impl From<EditorOutcome> for DocumentUpdate {
	fn from(outcome: EditorOutcome) -> Self {
		Self {
			changes: DocumentChanges::default()
				.with(DocumentChanges::DOCUMENT_CHANGED, outcome.document_changed())
				.with(DocumentChanges::VIEW_CHANGED, outcome.view_changed)
				.with(DocumentChanges::SELECTION_CHANGED, outcome.selection_changed)
				.with(DocumentChanges::MODE_CHANGED, outcome.mode_changed),
			viewport_target: outcome.viewport_target,
		}
	}
}

/// Owns the editable document and its synchronized presentation snapshot.
///
/// The session is the only place where mutations occur. The editor-facing
/// presentation is refreshed eagerly, while the heavier derived scene is built
/// only for inspect/perf/debug consumers.
pub(super) struct DocumentSession {
	editor: EditorEngine,
	editor_presentation: EditorPresentation,
	derived_scene: Option<DerivedScenePresentation>,
	next_derived_scene_revision: u64,
	font_system: FontSystem,
	#[cfg(test)]
	derived_scene_build_count: usize,
}

impl DocumentSession {
	/// Creates a new session seeded from the given text and scene config.
	pub(super) fn new(text: &str, config: SceneConfig) -> Self {
		let mut font_system = make_font_system();
		let editor = EditorEngine::new(&mut font_system, text, config);
		let editor_presentation = build_editor_presentation(&editor, 1);

		Self {
			editor,
			editor_presentation,
			derived_scene: None,
			next_derived_scene_revision: 0,
			font_system,
			#[cfg(test)]
			derived_scene_build_count: 0,
		}
	}

	/// Returns the latest synchronized editor presentation snapshot.
	pub(super) fn editor_presentation(&self) -> &EditorPresentation {
		&self.editor_presentation
	}

	/// Returns the latest derived scene snapshot if one is currently
	/// materialized.
	pub(super) fn derived_scene(&self) -> Option<&DerivedScenePresentation> {
		self.derived_scene.as_ref()
	}

	/// Returns the current document text.
	pub(super) fn text(&self) -> &str {
		self.editor.text()
	}

	/// Returns the current editor mode.
	pub(super) fn mode(&self) -> EditorMode {
		self.editor.mode()
	}

	/// Returns a clone of the current editor view state.
	#[cfg(test)]
	pub(super) fn view_state(&self) -> EditorViewState {
		self.editor_presentation.editor.clone()
	}

	/// Returns the latest measured viewport metrics.
	pub(super) fn viewport_metrics(&self) -> EditorViewportMetrics {
		self.editor_presentation.viewport_metrics
	}

	/// Returns the undo and redo stack depths.
	pub(super) fn history_depths(&self) -> (usize, usize) {
		self.editor.history_depths()
	}

	/// Applies a width change and refreshes the hot presentation if needed.
	pub(super) fn sync_width(&mut self, width: f32) -> bool {
		let changed = self.editor.sync_buffer_width(&mut self.font_system, width);
		self.finish_buffer_sync(changed)
	}

	/// Applies a config change and refreshes the hot presentation if needed.
	pub(super) fn sync_config(&mut self, config: SceneConfig) -> bool {
		let changed = self.editor.sync_buffer_config(&mut self.font_system, config);
		self.finish_buffer_sync(changed)
	}

	/// Replaces the document contents and refreshes the hot presentation.
	pub(super) fn reset_with_preset(&mut self, text: &str, config: SceneConfig) {
		self.editor.reset(&mut self.font_system, text, config);
		self.refresh_editor_presentation();
		self.invalidate_derived_scene();
	}

	/// Applies an editor intent and returns the app-facing change summary.
	pub(super) fn apply_editor_intent(&mut self, intent: EditorIntent) -> DocumentUpdate {
		let outcome = self.editor.apply(&mut self.font_system, intent);
		let update = DocumentUpdate::from(outcome);

		if update.changed() {
			self.refresh_editor_presentation();
		}

		if update.document_changed() {
			self.invalidate_derived_scene();
		}

		update
	}

	pub(super) fn ensure_derived_scene(&mut self) -> Option<Duration> {
		if self.derived_scene.is_some() {
			return None;
		}

		// The hot path is allowed to invalidate scene data aggressively because
		// scene consumers opt back in explicitly through this gate.
		let revision = self.next_derived_scene_revision + 1;
		let started = Instant::now();
		self.derived_scene = Some(build_derived_scene(&self.editor, revision));
		self.next_derived_scene_revision = revision;
		#[cfg(test)]
		{
			self.derived_scene_build_count += 1;
		}
		Some(started.elapsed())
	}

	pub(super) fn derived_scene_layout(&self) -> Option<&DocumentLayout> {
		self.derived_scene.as_ref().map(|scene| scene.layout.as_ref())
	}

	#[cfg(test)]
	pub(super) fn derived_scene_build_count(&self) -> usize {
		self.derived_scene_build_count
	}

	fn refresh_editor_presentation(&mut self) {
		// Keep the always-visible editor surface fully synchronized even when the
		// heavier inspect/perf scene stays cold.
		let revision = self.editor_presentation.revision + 1;
		self.editor_presentation = build_editor_presentation(&self.editor, revision);
	}

	fn finish_buffer_sync(&mut self, changed: bool) -> bool {
		if changed {
			self.refresh_editor_presentation();
			self.invalidate_derived_scene();
		}
		changed
	}

	fn invalidate_derived_scene(&mut self) {
		self.derived_scene = None;
	}
}

fn build_editor_presentation(editor: &EditorEngine, revision: u64) -> EditorPresentation {
	let text_layer = editor.text_layer_state();
	let viewport_metrics = editor.viewport_metrics();
	let editor_view = editor.view_state();

	EditorPresentation::new(revision, viewport_metrics, text_layer, editor_view, editor.text().len())
}

fn build_derived_scene(editor: &EditorEngine, revision: u64) -> DerivedScenePresentation {
	let layout = editor.shared_document_layout();
	DerivedScenePresentation::new(revision, layout)
}
