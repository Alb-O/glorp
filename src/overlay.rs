use iced::Point;

use std::sync::Arc;

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
	EditorSelection(EditorOverlayTone),
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
pub(crate) enum OverlayPrimitive {
	Rect {
		rect: LayoutRect,
		kind: OverlayRectKind,
		space: OverlaySpace,
	},
	Label {
		position: Point,
		kind: OverlayLabelKind,
		text: Arc<str>,
		space: OverlaySpace,
	},
}

impl OverlayPrimitive {
	pub(crate) fn scene_rect(rect: LayoutRect, kind: OverlayRectKind) -> Self {
		Self::Rect {
			rect,
			kind,
			space: OverlaySpace::Scene,
		}
	}

	pub(crate) fn viewport_label(position: Point, kind: OverlayLabelKind, text: impl Into<Arc<str>>) -> Self {
		Self::Label {
			position,
			kind,
			text: text.into(),
			space: OverlaySpace::Viewport,
		}
	}

	pub(crate) fn as_rect(&self) -> Option<(LayoutRect, OverlayRectKind, OverlaySpace)> {
		match self {
			Self::Rect { rect, kind, space } => Some((*rect, *kind, *space)),
			Self::Label { .. } => None,
		}
	}
}
