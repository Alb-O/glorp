use iced::widget::{button, container, row, text};
use iced::{Element, Length, Theme};

use crate::types::{Message, SidebarTab};

pub(crate) const SIDEBAR_WIDTH: f32 = 380.0;
const CONTROL_LABEL_WIDTH: f32 = 90.0;
pub(crate) const CONTROL_RADIUS: f32 = 6.0;
const CHECKBOX_RADIUS: f32 = 4.0;

pub(crate) fn control_row<'a>(label: impl Into<String>, control: Element<'a, Message>) -> Element<'a, Message> {
	row![text(label.into()).width(CONTROL_LABEL_WIDTH), control]
		.spacing(12)
		.align_y(iced::Center)
		.into()
}

pub(crate) fn panel_style(theme: &Theme) -> container::Style {
	let palette = theme.extended_palette();
	container::Style {
		background: Some(palette.background.weak.color.into()),
		border: iced::Border {
			color: palette.background.strong.color,
			width: 1.0,
			radius: CONTROL_RADIUS.into(),
		},
		..Default::default()
	}
}

pub(crate) fn surface_style(theme: &Theme) -> container::Style {
	let palette = theme.extended_palette();
	container::Style {
		background: Some(palette.background.base.color.into()),
		border: iced::Border {
			color: palette.background.strong.color,
			width: 1.0,
			radius: CONTROL_RADIUS.into(),
		},
		..Default::default()
	}
}

pub(crate) fn rounded_pick_list_style(
	theme: &Theme, status: iced::widget::pick_list::Status,
) -> iced::widget::pick_list::Style {
	let mut style = iced::widget::pick_list::default(theme, status);
	style.border.radius = CONTROL_RADIUS.into();
	style
}

pub(crate) fn rounded_pick_list_menu_style(theme: &Theme) -> iced::overlay::menu::Style {
	let mut style = iced::overlay::menu::default(theme);
	style.border.radius = CONTROL_RADIUS.into();
	style
}

pub(crate) fn rounded_checkbox_style(
	theme: &Theme, status: iced::widget::checkbox::Status,
) -> iced::widget::checkbox::Style {
	let mut style = iced::widget::checkbox::primary(theme, status);
	style.border.radius = CHECKBOX_RADIUS.into();
	style
}

pub(crate) fn rounded_text_editor_style(
	theme: &Theme, status: iced::widget::text_editor::Status,
) -> iced::widget::text_editor::Style {
	let mut style = iced::widget::text_editor::default(theme, status);
	style.border.radius = CONTROL_RADIUS.into();
	style
}

pub(crate) fn rounded_slider_style(theme: &Theme, status: iced::widget::slider::Status) -> iced::widget::slider::Style {
	let mut style = iced::widget::slider::default(theme, status);
	style.rail.border.radius = CONTROL_RADIUS.into();
	style
}

pub(crate) fn view_sidebar_tab(tab: SidebarTab, is_active: bool) -> Element<'static, Message> {
	let label_text = text(tab.to_string()).size(14).style(move |theme: &Theme| {
		let palette = theme.extended_palette();
		iced::widget::text::Style {
			color: Some(if is_active {
				palette.background.base.text
			} else {
				let mut color = palette.background.base.text;
				color.a = 0.82;
				color
			}),
		}
	});

	let indicator: Element<'static, Message> = if is_active {
		container(iced::widget::Space::new().width(Length::Fill).height(2))
			.style(move |theme: &Theme| {
				let palette = theme.extended_palette();
				container::Style {
					background: Some(palette.primary.base.color.into()),
					..Default::default()
				}
			})
			.into()
	} else {
		container(iced::widget::Space::new().width(Length::Fill).height(2)).into()
	};

	let content = iced::widget::column![
		indicator,
		container(
			button(
				container(label_text)
					.width(Length::Fill)
					.height(Length::Fill)
					.center_x(Length::Fill)
					.center_y(Length::Fill),
			)
			.on_press(Message::SelectSidebarTab(tab))
			.width(Length::Fill)
			.height(Length::Fill)
			.style(move |theme: &Theme, status| {
				let palette = theme.extended_palette();
				let mut overlay = palette.background.strong.color;
				overlay.a = if is_active {
					0.0
				} else {
					match status {
						button::Status::Hovered => 0.18,
						button::Status::Pressed => 0.24,
						_ => 0.0,
					}
				};

				button::Style {
					background: Some(overlay.into()),
					..button::text(theme, status)
				}
			})
		)
		.padding([3, 10])
		.height(Length::Fill)
		.width(Length::Fill)
	]
	.spacing(0);

	container(content)
		.height(38)
		.width(Length::Fill)
		.style(move |theme: &Theme| {
			let palette = theme.extended_palette();
			container::Style {
				background: Some(
					if is_active {
						palette.background.base.color
					} else {
						palette.background.weak.color
					}
					.into(),
				),
				border: iced::Border {
					width: 1.0,
					color: palette.background.strong.color,
					..iced::Border::default()
				},
				..Default::default()
			}
		})
		.into()
}
