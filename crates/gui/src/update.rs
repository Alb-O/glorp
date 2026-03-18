use {
	crate::GuiMessage,
	glorp_api::{GlorpExec, InspectTargetInput, PaneRatioInput, ScrollTarget, SidebarTabInput},
};

#[must_use]
pub const fn to_command(message: GuiMessage) -> GlorpExec {
	match message {
		GuiMessage::SidebarSelect(tab) => GlorpExec::UiSidebarSelect(SidebarTabInput { tab }),
		GuiMessage::InspectTargetSelect(target) => GlorpExec::UiInspectTargetSelect(InspectTargetInput { target }),
		GuiMessage::ViewportScrollTo { x, y } => GlorpExec::UiViewportScrollTo(ScrollTarget { x, y }),
		GuiMessage::PaneRatioSet(ratio) => GlorpExec::UiPaneRatioSet(PaneRatioInput { ratio }),
	}
}
