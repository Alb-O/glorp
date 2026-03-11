use iced::widget::{container, scrollable, text};
use iced::{Element, Font, Length};

use crate::types::Message;
use crate::ui::panel_style;

pub(crate) fn view_dump_tab<'a>(dump: &'a str) -> Element<'a, Message> {
	container(scrollable(text(dump).font(Font::MONOSPACE).size(14).width(Length::Fill)).height(Length::Fill))
		.padding(12)
		.height(Length::Fill)
		.style(panel_style)
		.into()
}
