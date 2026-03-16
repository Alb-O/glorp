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
}

/// Lazily-built scene snapshot used only by inspect/perf/debug consumers.
#[derive(Debug, Clone)]
pub(crate) struct DerivedScenePresentation {
	/// Monotonic revision used by scene caches and invalidation.
	pub(crate) revision: u64,
	/// Shared layout metadata for hit testing, inspection, and debug draw.
	pub(crate) layout: Arc<DocumentLayout>,
}

impl EditorPresentation {
	/// Builds a synchronized hot-path presentation from already-derived editor
	/// state.
	pub(crate) fn new(
		revision: u64, viewport_metrics: EditorViewportMetrics, text_layer: EditorTextLayerState,
		editor: EditorViewState, editor_bytes: usize,
	) -> Self {
		Self {
			revision,
			viewport_metrics,
			text_layer,
			editor,
			editor_bytes,
		}
	}

	pub(crate) fn mode(&self) -> EditorMode {
		self.editor.mode
	}

	pub(crate) fn editor_bytes(&self) -> usize {
		self.editor_bytes
	}
}

impl DerivedScenePresentation {
	pub(crate) fn new(revision: u64, layout: Arc<DocumentLayout>) -> Self {
		Self { revision, layout }
	}
}
