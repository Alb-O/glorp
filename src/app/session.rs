use {
	crate::{
		editor::{EditorEngine, EditorIntent, EditorMode, EditorOutcome, EditorViewportMetrics},
		overlay::LayoutRect,
		presentation::DocumentPresentation,
		scene::{DocumentLayout, SceneConfig, make_font_system},
	},
	cosmic_text::FontSystem,
};

#[cfg(test)]
use crate::editor::EditorViewState;

/// App-facing summary of what changed after applying an editor intent.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct DocumentUpdate {
	/// True when the document text changed.
	pub(crate) document_changed: bool,
	/// True when visible editor presentation changed.
	pub(crate) view_changed: bool,
	/// True when the logical selection changed.
	pub(crate) selection_changed: bool,
	/// True when the editor mode changed.
	pub(crate) mode_changed: bool,
	/// The latest viewport reveal target for the updated editor state.
	pub(crate) viewport_target: Option<LayoutRect>,
}

impl DocumentUpdate {
	/// Returns whether any app-visible editor state changed.
	pub(crate) fn changed(&self) -> bool {
		self.document_changed || self.view_changed || self.selection_changed || self.mode_changed
	}
}

impl From<EditorOutcome> for DocumentUpdate {
	fn from(outcome: EditorOutcome) -> Self {
		Self {
			document_changed: outcome.document_changed(),
			view_changed: outcome.view_changed,
			selection_changed: outcome.selection_changed,
			mode_changed: outcome.mode_changed,
			viewport_target: outcome.viewport_target,
		}
	}
}

/// Owns the editable document and its synchronized presentation snapshot.
///
/// The session is the only place where mutations occur. After each mutation it
/// refreshes either the full presentation or only the editor-facing slice,
/// depending on whether scene data actually changed.
pub(super) struct DocumentSession {
	editor: EditorEngine,
	presentation: DocumentPresentation,
	font_system: FontSystem,
}

impl DocumentSession {
	/// Creates a new session seeded from the given text and scene config.
	pub(super) fn new(text: &str, config: SceneConfig) -> Self {
		let mut font_system = make_font_system();
		let editor = EditorEngine::new(&mut font_system, text, config);
		let presentation = build_presentation(&editor, 1);

		Self {
			editor,
			presentation,
			font_system,
		}
	}

	/// Returns the latest synchronized presentation snapshot.
	pub(super) fn presentation(&self) -> &DocumentPresentation {
		&self.presentation
	}

	/// Returns the current document text.
	pub(super) fn text(&self) -> &str {
		self.editor.text()
	}

	/// Returns the current shared layout for the active presentation.
	pub(super) fn layout(&self) -> &DocumentLayout {
		self.presentation.layout.as_ref()
	}

	/// Returns the current editor mode.
	pub(super) fn mode(&self) -> EditorMode {
		self.editor.mode()
	}

	/// Returns a clone of the current editor view state.
	#[cfg(test)]
	pub(super) fn view_state(&self) -> EditorViewState {
		self.presentation.editor.clone()
	}

	/// Returns the latest measured viewport metrics.
	pub(super) fn viewport_metrics(&self) -> EditorViewportMetrics {
		self.presentation.viewport_metrics
	}

	/// Returns the undo and redo stack depths.
	pub(super) fn history_depths(&self) -> (usize, usize) {
		self.editor.history_depths()
	}

	/// Applies a width change and refreshes the full presentation if needed.
	pub(super) fn sync_width(&mut self, width: f32) -> bool {
		if !self.editor.sync_buffer_width(&mut self.font_system, width) {
			return false;
		}

		self.refresh_presentation();
		true
	}

	/// Applies a config change and refreshes the full presentation if needed.
	pub(super) fn sync_config(&mut self, config: SceneConfig) -> bool {
		if !self.editor.sync_buffer_config(&mut self.font_system, config) {
			return false;
		}

		self.refresh_presentation();
		true
	}

	/// Replaces the document contents and rebuilds the full presentation.
	pub(super) fn reset_with_preset(&mut self, text: &str, config: SceneConfig) {
		self.editor.reset(&mut self.font_system, text, config);
		self.refresh_presentation();
	}

	/// Applies an editor intent and returns the app-facing change summary.
	pub(super) fn apply_editor_intent(&mut self, intent: EditorIntent) -> DocumentUpdate {
		let outcome = self.editor.apply(&mut self.font_system, intent);
		let update = DocumentUpdate::from(outcome);

		if update.document_changed {
			self.refresh_presentation();
		} else if update.changed() {
			self.refresh_editor_view();
		}

		update
	}

	fn refresh_presentation(&mut self) {
		let revision = self.presentation.revision + 1;
		self.presentation = build_presentation(&self.editor, revision);
	}

	fn refresh_editor_view(&mut self) {
		// Selection/mode-only edits can reuse the existing scene metadata and
		// text buffer; only the editor-facing slice needs to move forward.
		self.presentation.revision += 1;
		self.presentation.editor = self.editor.view_state();
	}
}

/// Rebuilds the full layout/editor presentation from the editor snapshot.
fn build_presentation(editor: &EditorEngine, revision: u64) -> DocumentPresentation {
	let text_layer = editor.text_layer_state();
	let viewport_metrics = editor.viewport_metrics();
	let editor_view = editor.view_state();
	let layout = editor.shared_document_layout();

	DocumentPresentation::new(revision, viewport_metrics, text_layer, layout, editor_view)
}
