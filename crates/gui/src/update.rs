use {
	crate::GuiMessage,
	glorp_api::{GlorpCommand, UiCommand},
};

#[must_use]
pub const fn to_command(message: GuiMessage) -> GlorpCommand {
	GlorpCommand::Ui(match message {
		GuiMessage::SidebarSelect(tab) => UiCommand::SidebarSelect { tab },
		GuiMessage::InspectTargetSelect(target) => UiCommand::InspectTargetSelect { target },
		GuiMessage::ViewportScrollTo { x, y } => UiCommand::ViewportScrollTo { x, y },
		GuiMessage::PaneRatioSet(ratio) => UiCommand::PaneRatioSet { ratio },
	})
}
