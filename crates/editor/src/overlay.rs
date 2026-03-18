#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutRect {
	pub x: f32,
	pub y: f32,
	pub width: f32,
	pub height: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlaySpace {
	Scene,
	Viewport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayLayer {
	UnderText,
	OverText,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorOverlayTone {
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
pub enum OverlayRectKind {
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
pub enum OverlayLabelKind {
	SceneFooter,
	CanvasStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OverlayPrimitive {
	pub rect: LayoutRect,
	pub kind: OverlayRectKind,
	pub space: OverlaySpace,
	pub layer: OverlayLayer,
}

impl OverlayPrimitive {
	#[must_use]
	pub fn scene_rect(rect: LayoutRect, kind: OverlayRectKind, layer: OverlayLayer) -> Self {
		Self {
			rect,
			kind,
			space: OverlaySpace::Scene,
			layer,
		}
	}
}
