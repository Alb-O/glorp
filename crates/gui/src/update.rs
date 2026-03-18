use {
	crate::GuiMessage,
	glorp_api::{GlorpCommand, GlorpError, UiCommand},
};

pub fn to_command(message: GuiMessage) -> Result<GlorpCommand, GlorpError> {
	Ok(GlorpCommand::Ui(match message {
		GuiMessage::SidebarSelect(tab) => UiCommand::SidebarSelect { tab },
		GuiMessage::InspectTargetSelect(target) => UiCommand::InspectTargetSelect { target },
		GuiMessage::ViewportScrollTo { x, y } => UiCommand::ViewportScrollTo { x, y },
		GuiMessage::PaneRatioSet(ratio) => UiCommand::PaneRatioSet { ratio },
	}))
}
