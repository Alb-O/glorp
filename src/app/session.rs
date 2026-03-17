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

/// Controls whether session operations must keep derived scene data hot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SceneDemand {
	HotOnly,
	DerivedRequired,
}

impl SceneDemand {
	fn rebuilds_existing_scene(self) -> bool {
		matches!(self, Self::DerivedRequired)
	}

	fn requires_scene(self) -> bool {
		matches!(self, Self::DerivedRequired)
	}
}

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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum ScrollIntent {
	#[default]
	KeepClamped,
	ResetScroll,
}

/// App-facing summary of what changed after a session operation.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct SessionUpdate {
	pub(crate) changes: DocumentChanges,
	pub(crate) scroll_intent: ScrollIntent,
	pub(crate) viewport_target: Option<LayoutRect>,
	pub(crate) width_sync: Option<Duration>,
	pub(crate) scene_build: Option<Duration>,
}

impl SessionUpdate {
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

impl From<EditorOutcome> for SessionUpdate {
	fn from(outcome: EditorOutcome) -> Self {
		Self {
			changes: DocumentChanges::default()
				.with(DocumentChanges::DOCUMENT_CHANGED, outcome.document_changed())
				.with(DocumentChanges::VIEW_CHANGED, outcome.view_changed)
				.with(DocumentChanges::SELECTION_CHANGED, outcome.selection_changed)
				.with(DocumentChanges::MODE_CHANGED, outcome.mode_changed),
			scroll_intent: ScrollIntent::KeepClamped,
			viewport_target: outcome.viewport_target,
			width_sync: None,
			scene_build: None,
		}
	}
}

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

	#[cfg(test)]
	pub(super) fn history_depths(&self) -> (usize, usize) {
		(self.snapshot.editor.undo_depth, self.snapshot.editor.redo_depth)
	}

	pub(super) fn sync_width(&mut self, width: f32, demand: SceneDemand) -> SessionUpdate {
		let started = Instant::now();
		let changed = self.editor.sync_buffer_width(&mut self.font_system, width);
		let width_sync = changed.then(|| started.elapsed());
		self.finish_buffer_sync(changed, demand, ScrollIntent::KeepClamped, width_sync)
	}

	pub(super) fn sync_config(&mut self, config: SceneConfig, demand: SceneDemand) -> SessionUpdate {
		let changed = self.editor.sync_buffer_config(&mut self.font_system, config);
		self.finish_buffer_sync(changed, demand, ScrollIntent::ResetScroll, None)
	}

	pub(super) fn reset_with_preset(&mut self, text: &str, config: SceneConfig, demand: SceneDemand) -> SessionUpdate {
		self.editor.reset(&mut self.font_system, text, config);
		self.refresh_editor_snapshot();
		self.invalidate_scene();

		SessionUpdate {
			changes: DocumentChanges::default()
				.with(DocumentChanges::DOCUMENT_CHANGED, true)
				.with(DocumentChanges::VIEW_CHANGED, true)
				.with(DocumentChanges::SELECTION_CHANGED, true)
				.with(DocumentChanges::MODE_CHANGED, true),
			scroll_intent: ScrollIntent::ResetScroll,
			viewport_target: self.snapshot.editor.editor.viewport_target,
			width_sync: None,
			scene_build: self.reconcile_scene(demand, true),
		}
	}

	pub(super) fn apply_editor_intent(&mut self, intent: EditorIntent, demand: SceneDemand) -> SessionUpdate {
		let outcome = self.editor.apply(&mut self.font_system, intent);
		let mut update = SessionUpdate::from(outcome);

		if update.changed() {
			self.refresh_editor_snapshot();
		}

		if update.document_changed() {
			self.invalidate_scene();
			update.scene_build = self.reconcile_scene(demand, true);
		} else {
			update.scene_build = self.reconcile_scene(demand, false);
		}

		update
	}

	pub(super) fn ensure_scene(&mut self, demand: SceneDemand) -> SessionUpdate {
		SessionUpdate {
			scene_build: self.reconcile_scene(demand, false),
			..SessionUpdate::default()
		}
	}

	#[cfg(test)]
	pub(super) fn derived_scene_build_count(&self) -> usize {
		self.derived_scene_build_count
	}

	fn refresh_editor_snapshot(&mut self) {
		let revision = self.snapshot.editor.revision + 1;
		self.snapshot.editor = build_editor_presentation(&self.editor, revision);
	}

	fn finish_buffer_sync(
		&mut self, changed: bool, demand: SceneDemand, scroll_intent: ScrollIntent, width_sync: Option<Duration>,
	) -> SessionUpdate {
		if !changed {
			return SessionUpdate {
				scroll_intent,
				width_sync,
				scene_build: self.reconcile_scene(demand, false),
				..SessionUpdate::default()
			};
		}

		self.refresh_editor_snapshot();
		self.invalidate_scene();

		SessionUpdate {
			changes: DocumentChanges::default()
				.with(DocumentChanges::DOCUMENT_CHANGED, true)
				.with(DocumentChanges::VIEW_CHANGED, true),
			scroll_intent,
			viewport_target: self.snapshot.editor.editor.viewport_target,
			width_sync,
			scene_build: self.reconcile_scene(demand, true),
		}
	}

	fn reconcile_scene(&mut self, demand: SceneDemand, rebuild_requested: bool) -> Option<Duration> {
		let rebuild_existing = demand.rebuilds_existing_scene() && rebuild_requested && self.snapshot.scene.is_some();
		let cold_build = demand.requires_scene() && self.snapshot.scene.is_none();
		if !rebuild_existing && !cold_build {
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
