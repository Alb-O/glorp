//! Read-only presentation state shared by rendering, sidebar inspection, and
//! viewport logic.

use {
	crate::{
		editor::{EditorMode, EditorTextLayerState, EditorViewState, EditorViewportMetrics},
		overlay::OverlayPrimitive,
		scene::DocumentLayout,
		types::CanvasTarget,
	},
	std::sync::Arc,
};

/// Canonical derived state for the current document revision.
///
/// This bundles the editor-facing view state and the scene-facing layout data
/// so consumers can render from one synchronized snapshot instead of
/// coordinating separate models.
#[derive(Debug, Clone)]
pub(crate) struct DocumentPresentation {
	/// Monotonic revision used by caches and view invalidation.
	pub(crate) revision: u64,
	/// Measured content size used for scroll clamping and viewport reveal.
	pub(crate) viewport_metrics: EditorViewportMetrics,
	/// Shared text buffer handle for the text renderer.
	pub(crate) text_layer: EditorTextLayerState,
	/// Shared layout metadata for hit testing, inspection, and decoration draw.
	pub(crate) layout: Arc<DocumentLayout>,
	/// Editor mode, selection, overlays, and viewport target for the same layout.
	pub(crate) editor: EditorViewState,
}

impl DocumentPresentation {
	/// Builds a synchronized presentation from already-derived editor and layout
	/// state.
	pub(crate) fn new(
		revision: u64, viewport_metrics: EditorViewportMetrics, text_layer: EditorTextLayerState,
		layout: Arc<DocumentLayout>, editor: EditorViewState,
	) -> Self {
		Self {
			revision,
			viewport_metrics,
			text_layer,
			layout,
			editor,
		}
	}

	/// Returns the current document text as rendered by the layout.
	pub(crate) fn text(&self) -> &str {
		self.layout.text.as_ref()
	}

	/// Returns the active editor mode for this presentation revision.
	pub(crate) fn mode(&self) -> EditorMode {
		self.editor.mode
	}

	/// Returns the current document length in bytes.
	pub(crate) fn editor_bytes(&self) -> usize {
		self.layout.text.len()
	}

	/// Builds transient inspect overlays without mutating the underlying layout.
	pub(crate) fn inspect_overlay_primitives(
		&self, hovered_target: Option<CanvasTarget>, selected_target: Option<CanvasTarget>, layout_width: f32,
		show_hitboxes: bool,
	) -> Arc<[OverlayPrimitive]> {
		self.layout
			.inspect_overlay_primitives(hovered_target, selected_target, layout_width, show_hitboxes)
	}
}
