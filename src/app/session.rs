use {
	crate::{
		editor::{EditorEngine, EditorIntent, EditorMode, EditorOutcome},
		presentation::{EditorPresentation, ScenePresentation, SessionSnapshot},
		scene::{SceneConfig, make_font_system},
	},
	cosmic_text::FontSystem,
	std::time::{Duration, Instant},
};

#[cfg(test)]
use crate::{
	editor::{EditorEditIntent, EditorModeIntent, EditorViewState},
	scene::scene_config,
	types::{FontChoice, ShapingChoice, WrapChoice},
};

#[derive(Debug, Clone, PartialEq)]
pub(super) enum SessionRequest {
	ReplaceDocument { text: String, config: SceneConfig },
	SyncConfig(SceneConfig),
	SyncWidth(f32),
	ApplyEditorIntent(EditorIntent),
	EnsureScene,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SceneDemand {
	HotPathOnly,
	DerivedScene,
}

impl SceneDemand {
	const fn materializes_scene(self) -> bool {
		matches!(self, Self::DerivedScene)
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SessionChange {
	Text,
	View,
	Selection,
	Mode,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct SessionChanges(u8);

impl SessionChange {
	const fn bit(self) -> u8 {
		match self {
			Self::Text => 1 << 0,
			Self::View => 1 << 1,
			Self::Selection => 1 << 2,
			Self::Mode => 1 << 3,
		}
	}
}

impl SessionChanges {
	const fn with(mut self, change: SessionChange) -> Self {
		self.0 |= change.bit();
		self
	}

	const fn with_if(self, condition: bool, change: SessionChange) -> Self {
		if condition { self.with(change) } else { self }
	}

	const fn contains(self, change: SessionChange) -> bool {
		self.0 & change.bit() != 0
	}

	const fn is_empty(self) -> bool {
		self.0 == 0
	}
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct SessionDelta {
	changes: SessionChanges,
	pub(super) width_sync: Option<Duration>,
	pub(super) scene_materialized: Option<Duration>,
}

#[derive(Debug, Clone)]
pub(super) struct SessionFeedback {
	pub(super) delta: SessionDelta,
	pub(super) snapshot: SessionSnapshot,
}

impl SessionDelta {
	pub(super) fn changed(&self) -> bool {
		!self.changes.is_empty() || self.width_sync.is_some() || self.scene_materialized.is_some()
	}

	pub(super) fn document_changed(&self) -> bool {
		self.text_changed()
	}

	pub(super) fn text_changed(&self) -> bool {
		self.changes.contains(SessionChange::Text)
	}

	pub(super) fn view_changed(&self) -> bool {
		self.changes.contains(SessionChange::View)
	}

	pub(super) fn selection_changed(&self) -> bool {
		self.changes.contains(SessionChange::Selection)
	}

	pub(super) fn mode_changed(&self) -> bool {
		self.changes.contains(SessionChange::Mode)
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

	#[cfg(test)]
	pub(super) fn history_depths(&self) -> (usize, usize) {
		(self.snapshot.editor.undo_depth, self.snapshot.editor.redo_depth)
	}

	pub(super) fn execute(&mut self, request: SessionRequest, demand: SceneDemand) -> SessionFeedback {
		let ensure_scene = matches!(request, SessionRequest::EnsureScene);
		let mut delta = match request {
			SessionRequest::ReplaceDocument { text, config } => self.execute_replace_document(&text, config),
			SessionRequest::SyncConfig(config) => self.execute_sync_config(config),
			SessionRequest::SyncWidth(width) => self.execute_sync_width(width),
			SessionRequest::ApplyEditorIntent(intent) => self.execute_editor_intent(intent),
			SessionRequest::EnsureScene => SessionDelta::default(),
		};

		if demand.materializes_scene() || ensure_scene {
			delta.scene_materialized = self.materialize_scene_if_needed();
		}

		SessionFeedback {
			delta,
			snapshot: self.snapshot.clone(),
		}
	}

	#[cfg(test)]
	pub(super) fn derived_scene_build_count(&self) -> usize {
		self.derived_scene_build_count
	}

	fn execute_replace_document(&mut self, text: &str, config: SceneConfig) -> SessionDelta {
		self.editor.reset(&mut self.font_system, text, config);
		self.refresh_editor_snapshot();
		self.invalidate_scene();
		SessionDelta {
			changes: SessionChanges::default()
				.with(SessionChange::Text)
				.with(SessionChange::View)
				.with(SessionChange::Selection)
				.with(SessionChange::Mode),
			..SessionDelta::default()
		}
	}

	fn execute_sync_config(&mut self, config: SceneConfig) -> SessionDelta {
		if !self.editor.sync_buffer_config(&mut self.font_system, config) {
			return SessionDelta::default();
		}

		self.refresh_editor_snapshot();
		self.invalidate_scene();
		SessionDelta {
			changes: SessionChanges::default().with(SessionChange::View),
			..SessionDelta::default()
		}
	}

	fn execute_sync_width(&mut self, width: f32) -> SessionDelta {
		let started = Instant::now();
		if !self.editor.sync_buffer_width(&mut self.font_system, width) {
			return SessionDelta::default();
		}

		self.refresh_editor_snapshot();
		self.invalidate_scene();
		SessionDelta {
			changes: SessionChanges::default().with(SessionChange::View),
			width_sync: Some(started.elapsed()),
			..SessionDelta::default()
		}
	}

	fn execute_editor_intent(&mut self, intent: EditorIntent) -> SessionDelta {
		let EditorOutcome {
			view_changed,
			selection_changed,
			mode_changed,
			viewport_target: _,
			text_edit,
		} = self.editor.apply(&mut self.font_system, intent);
		let text_changed = text_edit.is_some();

		if view_changed || selection_changed || mode_changed || text_changed {
			self.refresh_editor_snapshot();
		}

		if text_changed {
			self.invalidate_scene();
		}

		SessionDelta {
			changes: SessionChanges::default()
				.with_if(text_changed, SessionChange::Text)
				.with_if(view_changed, SessionChange::View)
				.with_if(selection_changed, SessionChange::Selection)
				.with_if(mode_changed, SessionChange::Mode),
			..SessionDelta::default()
		}
	}

	fn refresh_editor_snapshot(&mut self) {
		let revision = self.snapshot.editor.revision + 1;
		self.snapshot.editor = build_editor_presentation(&self.editor, revision);
	}

	fn invalidate_scene(&mut self) {
		self.snapshot.scene = None;
	}

	fn materialize_scene_if_needed(&mut self) -> Option<Duration> {
		if self.snapshot.scene.is_some() {
			return None;
		}

		let revision = self.next_scene_revision + 1;
		let started = Instant::now();
		self.snapshot.scene = Some(ScenePresentation::new(revision, self.editor.shared_document_layout()));
		self.next_scene_revision = revision;
		#[cfg(test)]
		{
			self.derived_scene_build_count += 1;
		}
		Some(started.elapsed())
	}
}

fn build_editor_presentation(editor: &EditorEngine, revision: u64) -> EditorPresentation {
	let (undo_depth, redo_depth) = editor.history_depths();
	EditorPresentation::new(
		revision,
		editor.viewport_metrics(),
		editor.text_layer_state(),
		editor.view_state(),
		editor.text().len(),
		undo_depth,
		redo_depth,
	)
}

#[cfg(test)]
fn test_config(width: f32) -> SceneConfig {
	scene_config(
		FontChoice::JetBrainsMono,
		ShapingChoice::Advanced,
		WrapChoice::Word,
		24.0,
		32.0,
		width,
	)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn text_edit_without_scene_demand_invalidates_scene_without_rebuilding() {
		let mut session = DocumentSession::new("abc", test_config(540.0));
		let initial = session.execute(SessionRequest::EnsureScene, SceneDemand::DerivedScene);
		assert!(initial.delta.scene_materialized.is_some());
		assert_eq!(session.derived_scene_build_count(), 1);

		session.execute(
			SessionRequest::ApplyEditorIntent(EditorIntent::Mode(EditorModeIntent::EnterInsertAfter)),
			SceneDemand::HotPathOnly,
		);
		let transition = session.execute(
			SessionRequest::ApplyEditorIntent(EditorIntent::Edit(EditorEditIntent::InsertText("!".to_string()))),
			SceneDemand::HotPathOnly,
		);

		assert!(transition.delta.document_changed());
		assert!(transition.delta.scene_materialized.is_none());
		assert!(session.snapshot().scene.is_none());
		assert_eq!(session.derived_scene_build_count(), 1);
	}

	#[test]
	fn text_edit_with_scene_demand_rebuilds_scene_once() {
		let mut session = DocumentSession::new("abc", test_config(540.0));
		session.execute(SessionRequest::EnsureScene, SceneDemand::DerivedScene);
		assert_eq!(session.derived_scene_build_count(), 1);

		session.execute(
			SessionRequest::ApplyEditorIntent(EditorIntent::Mode(EditorModeIntent::EnterInsertAfter)),
			SceneDemand::DerivedScene,
		);
		assert_eq!(session.derived_scene_build_count(), 1);

		let transition = session.execute(
			SessionRequest::ApplyEditorIntent(EditorIntent::Edit(EditorEditIntent::InsertText("!".to_string()))),
			SceneDemand::DerivedScene,
		);

		assert!(transition.delta.document_changed());
		assert!(transition.delta.scene_materialized.is_some());
		assert_eq!(session.derived_scene_build_count(), 2);
		assert!(session.snapshot().scene.is_some());
	}

	#[test]
	fn sync_config_no_op_returns_empty_transition() {
		let mut session = DocumentSession::new("abc", test_config(540.0));

		let transition = session.execute(SessionRequest::SyncConfig(test_config(540.0)), SceneDemand::HotPathOnly);

		assert!(!transition.delta.changed());
		assert!(transition.delta.scene_materialized.is_none());
	}

	#[test]
	fn width_sync_reports_duration_only_for_real_width_changes() {
		let mut session = DocumentSession::new("abc", test_config(540.0));

		let noop = session.execute(SessionRequest::SyncWidth(540.0), SceneDemand::HotPathOnly);
		assert!(noop.delta.width_sync.is_none());
		assert!(!noop.delta.changed());

		let changed = session.execute(SessionRequest::SyncWidth(640.0), SceneDemand::HotPathOnly);
		assert!(changed.delta.width_sync.is_some());
		assert!(changed.delta.view_changed());
	}

	#[test]
	fn ensure_scene_is_a_no_op_when_already_materialized() {
		let mut session = DocumentSession::new("abc", test_config(540.0));

		let initial = session.execute(SessionRequest::EnsureScene, SceneDemand::DerivedScene);
		let repeated = session.execute(SessionRequest::EnsureScene, SceneDemand::DerivedScene);

		assert!(initial.delta.scene_materialized.is_some());
		assert!(repeated.delta.scene_materialized.is_none());
		assert_eq!(session.derived_scene_build_count(), 1);
	}
}
