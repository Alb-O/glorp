//! Read-only presentation state shared by rendering, sidebar inspection, and
//! viewport logic.

use {
	crate::{
		editor::{EditorMode, EditorTextLayerState, EditorViewState, EditorViewportMetrics},
		scene::DocumentLayout,
	},
	std::sync::Arc,
};

/// Always-hot editor state shared by the main edit/render path.
#[derive(Debug, Clone)]
pub(crate) struct EditorPresentation {
	/// Monotonic revision used by caches and view invalidation on the hot path.
	pub(crate) revision: u64,
	/// Measured content size used for scroll clamping and viewport reveal.
	pub(crate) viewport_metrics: EditorViewportMetrics,
	/// Shared text buffer handle for the text renderer.
	pub(crate) text_layer: EditorTextLayerState,
	/// Editor mode, selection, overlays, and viewport target for the hot path.
	pub(crate) editor: EditorViewState,
	/// Current document size in bytes.
	pub(crate) editor_bytes: usize,
	/// Current undo stack depth.
	pub(crate) undo_depth: usize,
	/// Current redo stack depth.
	pub(crate) redo_depth: usize,
}

/// Lazily-built scene snapshot used only by inspect/perf/debug consumers.
#[derive(Debug, Clone)]
pub(crate) struct ScenePresentation {
	/// Monotonic revision used by scene caches and invalidation.
	pub(crate) revision: u64,
	/// Shared layout metadata for hit testing, inspection, and debug draw.
	pub(crate) layout: Arc<DocumentLayout>,
}

/// Coherent session-owned presentation state for the current frame.
#[derive(Debug, Clone)]
pub(crate) struct SessionSnapshot {
	pub(crate) editor: EditorPresentation,
	pub(crate) scene: Option<ScenePresentation>,
}

impl EditorPresentation {
	/// Builds a synchronized hot-path presentation from already-derived editor
	/// state.
	pub(crate) fn new(
		revision: u64, viewport_metrics: EditorViewportMetrics, text_layer: EditorTextLayerState,
		editor: EditorViewState, editor_bytes: usize, undo_depth: usize, redo_depth: usize,
	) -> Self {
		Self {
			revision,
			viewport_metrics,
			text_layer,
			editor,
			editor_bytes,
			undo_depth,
			redo_depth,
		}
	}

	pub(crate) fn mode(&self) -> EditorMode {
		self.editor.mode
	}

	pub(crate) fn editor_bytes(&self) -> usize {
		self.editor_bytes
	}
}

impl ScenePresentation {
	pub(crate) fn new(revision: u64, layout: Arc<DocumentLayout>) -> Self {
		Self { revision, layout }
	}
}

impl SessionSnapshot {
	pub(crate) fn new(editor: EditorPresentation) -> Self {
		Self { editor, scene: None }
	}

	pub(crate) fn mode(&self) -> EditorMode {
		self.editor.mode()
	}

	pub(crate) fn editor_bytes(&self) -> usize {
		self.editor.editor_bytes()
	}
}
