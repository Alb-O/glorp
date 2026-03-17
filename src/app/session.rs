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
pub(super) enum SessionCommand {
	ReplaceDocument { text: String, config: SceneConfig },
	SyncConfig(SceneConfig),
	SyncWidth(f32),
	ApplyEditorIntent(EditorIntent),
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct SessionTransition {
	pub(super) text_changed: bool,
	pub(super) view_changed: bool,
	pub(super) selection_changed: bool,
	pub(super) mode_changed: bool,
	pub(super) width_sync: Option<Duration>,
	pub(super) scene_materialized: Option<Duration>,
}

impl SessionTransition {
	pub(super) fn changed(&self) -> bool {
		self.text_changed
			|| self.view_changed
			|| self.selection_changed
			|| self.mode_changed
			|| self.width_sync.is_some()
			|| self.scene_materialized.is_some()
	}

	pub(super) fn document_changed(&self) -> bool {
		self.text_changed
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

	pub(super) fn execute(&mut self, command: SessionCommand, materialize_scene: bool) -> SessionTransition {
		let mut transition = match command {
			SessionCommand::ReplaceDocument { text, config } => self.execute_replace_document(&text, config),
			SessionCommand::SyncConfig(config) => self.execute_sync_config(config),
			SessionCommand::SyncWidth(width) => self.execute_sync_width(width),
			SessionCommand::ApplyEditorIntent(intent) => self.execute_editor_intent(intent),
		};

		if materialize_scene {
			transition.scene_materialized = self.materialize_scene_if_needed();
		}

		transition
	}

	pub(super) fn ensure_scene_materialized(&mut self) -> SessionTransition {
		SessionTransition {
			scene_materialized: self.materialize_scene_if_needed(),
			..SessionTransition::default()
		}
	}

	#[cfg(test)]
	pub(super) fn derived_scene_build_count(&self) -> usize {
		self.derived_scene_build_count
	}

	fn execute_replace_document(&mut self, text: &str, config: SceneConfig) -> SessionTransition {
		self.editor.reset(&mut self.font_system, text, config);
		self.refresh_editor_snapshot();
		self.invalidate_scene();
		SessionTransition {
			text_changed: true,
			view_changed: true,
			selection_changed: true,
			mode_changed: true,
			..SessionTransition::default()
		}
	}

	fn execute_sync_config(&mut self, config: SceneConfig) -> SessionTransition {
		if !self.editor.sync_buffer_config(&mut self.font_system, config) {
			return SessionTransition::default();
		}

		self.refresh_editor_snapshot();
		self.invalidate_scene();
		SessionTransition {
			view_changed: true,
			..SessionTransition::default()
		}
	}

	fn execute_sync_width(&mut self, width: f32) -> SessionTransition {
		let started = Instant::now();
		if !self.editor.sync_buffer_width(&mut self.font_system, width) {
			return SessionTransition::default();
		}

		self.refresh_editor_snapshot();
		self.invalidate_scene();
		SessionTransition {
			view_changed: true,
			width_sync: Some(started.elapsed()),
			..SessionTransition::default()
		}
	}

	fn execute_editor_intent(&mut self, intent: EditorIntent) -> SessionTransition {
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

		SessionTransition {
			text_changed,
			view_changed,
			selection_changed,
			mode_changed,
			..SessionTransition::default()
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
		let initial = session.ensure_scene_materialized();
		assert!(initial.scene_materialized.is_some());
		assert_eq!(session.derived_scene_build_count(), 1);

		session.execute(
			SessionCommand::ApplyEditorIntent(EditorIntent::Mode(EditorModeIntent::EnterInsertAfter)),
			false,
		);
		let transition = session.execute(
			SessionCommand::ApplyEditorIntent(EditorIntent::Edit(EditorEditIntent::InsertText("!".to_string()))),
			false,
		);

		assert!(transition.document_changed());
		assert!(transition.scene_materialized.is_none());
		assert!(session.snapshot().scene.is_none());
		assert_eq!(session.derived_scene_build_count(), 1);
	}

	#[test]
	fn text_edit_with_scene_demand_rebuilds_scene_once() {
		let mut session = DocumentSession::new("abc", test_config(540.0));
		session.ensure_scene_materialized();
		assert_eq!(session.derived_scene_build_count(), 1);

		session.execute(
			SessionCommand::ApplyEditorIntent(EditorIntent::Mode(EditorModeIntent::EnterInsertAfter)),
			true,
		);
		assert_eq!(session.derived_scene_build_count(), 1);

		let transition = session.execute(
			SessionCommand::ApplyEditorIntent(EditorIntent::Edit(EditorEditIntent::InsertText("!".to_string()))),
			true,
		);

		assert!(transition.document_changed());
		assert!(transition.scene_materialized.is_some());
		assert_eq!(session.derived_scene_build_count(), 2);
		assert!(session.snapshot().scene.is_some());
	}

	#[test]
	fn sync_config_no_op_returns_empty_transition() {
		let mut session = DocumentSession::new("abc", test_config(540.0));

		let transition = session.execute(SessionCommand::SyncConfig(test_config(540.0)), false);

		assert!(!transition.changed());
		assert!(transition.scene_materialized.is_none());
	}

	#[test]
	fn width_sync_reports_duration_only_for_real_width_changes() {
		let mut session = DocumentSession::new("abc", test_config(540.0));

		let noop = session.execute(SessionCommand::SyncWidth(540.0), false);
		assert!(noop.width_sync.is_none());
		assert!(!noop.changed());

		let changed = session.execute(SessionCommand::SyncWidth(640.0), false);
		assert!(changed.width_sync.is_some());
		assert!(changed.view_changed);
	}

	#[test]
	fn ensure_scene_is_a_no_op_when_already_materialized() {
		let mut session = DocumentSession::new("abc", test_config(540.0));

		let initial = session.ensure_scene_materialized();
		let repeated = session.ensure_scene_materialized();

		assert!(initial.scene_materialized.is_some());
		assert!(repeated.scene_materialized.is_none());
		assert_eq!(session.derived_scene_build_count(), 1);
	}
}
