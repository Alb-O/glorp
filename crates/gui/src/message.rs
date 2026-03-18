use glorp_api::{CanvasTarget, SidebarTab};

#[derive(Debug, Clone, PartialEq)]
pub enum GuiMessage {
	SidebarSelect(SidebarTab),
	InspectTargetSelect(Option<CanvasTarget>),
	ViewportScrollTo { x: f32, y: f32 },
	PaneRatioSet(f32),
}
