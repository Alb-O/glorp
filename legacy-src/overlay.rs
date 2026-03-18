#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LayoutRect {
	pub(crate) x: f32,
	pub(crate) y: f32,
	pub(crate) width: f32,
	pub(crate) height: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OverlaySpace {
	Scene,
	Viewport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OverlayLayer {
	UnderText,
	OverText,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EditorOverlayTone {
	Normal,
	Insert,
}

impl From<crate::editor::EditorMode> for EditorOverlayTone {
	fn from(value: crate::editor::EditorMode) -> Self {
		match value {
			crate::editor::EditorMode::Normal => Self::Normal,
			crate::editor::EditorMode::Insert => Self::Insert,
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OverlayRectKind {
	EditorSelection,
	EditorActive(EditorOverlayTone),
	EditorInsertBlock(EditorOverlayTone),
	EditorCaret(EditorOverlayTone),
	EditorFocusFrame(EditorOverlayTone),
	InspectRunHover,
	InspectRunSelected,
	InspectGlyphHover,
	InspectGlyphSelected,
	InspectGlyphHitboxHover,
	InspectGlyphHitboxSelected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OverlayLabelKind {
	SceneFooter,
	CanvasStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct OverlayPrimitive {
	pub(crate) rect: LayoutRect,
	pub(crate) kind: OverlayRectKind,
	pub(crate) space: OverlaySpace,
	pub(crate) layer: OverlayLayer,
}

impl OverlayPrimitive {
	pub(crate) fn scene_rect(rect: LayoutRect, kind: OverlayRectKind, layer: OverlayLayer) -> Self {
		Self {
			rect,
			kind,
			space: OverlaySpace::Scene,
			layer,
		}
	}
}
