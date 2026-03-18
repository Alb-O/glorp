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
pub struct EditorPresentation {
	/// Monotonic revision used by caches and view invalidation on the hot path.
	pub revision: u64,
	/// Measured content size used for scroll clamping and viewport reveal.
	pub viewport_metrics: EditorViewportMetrics,
	/// Shared text buffer handle for the text renderer.
	pub text_layer: EditorTextLayerState,
	/// Editor mode, selection, overlays, and viewport target for the hot path.
	pub editor: EditorViewState,
	/// Current document size in bytes.
	pub editor_bytes: usize,
	/// Current undo stack depth.
	pub undo_depth: usize,
	/// Current redo stack depth.
	pub redo_depth: usize,
}

/// Lazily-built scene snapshot used only by inspect/perf/debug consumers.
#[derive(Debug, Clone)]
pub struct ScenePresentation {
	/// Monotonic revision used by scene caches and invalidation.
	pub revision: u64,
	/// Shared layout metadata for hit testing, inspection, and debug draw.
	pub layout: Arc<DocumentLayout>,
}

/// Coherent session-owned presentation state for the current frame.
#[derive(Debug, Clone)]
pub struct SessionSnapshot {
	pub editor: EditorPresentation,
	pub scene: Option<ScenePresentation>,
}

impl EditorPresentation {
	/// Builds a synchronized hot-path presentation from already-derived editor
	/// state.
	#[must_use]
	pub const fn new(
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
}

impl ScenePresentation {
	#[must_use]
	pub const fn new(revision: u64, layout: Arc<DocumentLayout>) -> Self {
		Self { revision, layout }
	}
}

impl SessionSnapshot {
	#[must_use]
	pub const fn new(editor: EditorPresentation) -> Self {
		Self { editor, scene: None }
	}

	#[must_use]
	pub const fn mode(&self) -> EditorMode {
		self.editor.editor.mode
	}

	#[must_use]
	pub const fn editor_bytes(&self) -> usize {
		self.editor.editor_bytes
	}
}
