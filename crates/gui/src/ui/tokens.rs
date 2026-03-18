use {
	crate::types::{Message, SidebarTab, sidebar_tab_label},
	iced::{
		Element, Length, Theme,
		widget::{button, container, row, scrollable, text},
	},
};

pub(crate) const SIDEBAR_WIDTH: f32 = 380.0;
const CONTROL_LABEL_WIDTH: f32 = 90.0;
pub(crate) const CONTROL_RADIUS: f32 = 6.0;
pub(crate) const PICK_LIST_PADDING: [u16; 2] = [8, 12];
const CHECKBOX_RADIUS: f32 = 4.0;
const PANEL_SCROLLBAR_WIDTH: f32 = 8.0;
const PANEL_SCROLLER_WIDTH: f32 = 8.0;
const PANEL_SCROLLBAR_GAP: f32 = 10.0;

pub(crate) fn control_row(label: impl Into<String>, control: Element<'_, Message>) -> Element<'_, Message> {
	row![text(label.into()).width(CONTROL_LABEL_WIDTH), control]
		.spacing(12)
		.align_y(iced::Center)
		.into()
}

pub(crate) fn panel_style(theme: &Theme) -> container::Style {
	let palette = theme.palette();
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

pub(crate) fn panel_scrollable<'a>(content: impl Into<Element<'a, Message>>) -> iced::widget::Scrollable<'a, Message> {
	scrollable(content)
		.width(Length::Fill)
		.direction(scrollable::Direction::Vertical(
			scrollable::Scrollbar::new()
				.width(PANEL_SCROLLBAR_WIDTH)
				.scroller_width(PANEL_SCROLLER_WIDTH)
				.spacing(PANEL_SCROLLBAR_GAP),
		))
		.style(panel_scrollable_style)
}

pub(crate) fn surface_style(theme: &Theme) -> container::Style {
	let palette = theme.palette();
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

pub(crate) fn panel_scrollable_style(
	theme: &Theme, status: iced::widget::scrollable::Status,
) -> iced::widget::scrollable::Style {
	let palette = theme.palette();
	let mut style = iced::widget::scrollable::default(theme, status);

	let idle = (0.10, 0.38, palette.background.strongest.color, 0.0);
	let (rail_alpha, handle_alpha, handle_color, border_alpha) = match status {
		iced::widget::scrollable::Status::Active { .. } => idle,
		iced::widget::scrollable::Status::Hovered {
			is_vertical_scrollbar_hovered,
			..
		} => {
			if is_vertical_scrollbar_hovered {
				(0.18, 0.92, palette.primary.strong.color, 0.35)
			} else {
				idle
			}
		}
		iced::widget::scrollable::Status::Dragged {
			is_vertical_scrollbar_dragged,
			..
		} => {
			if is_vertical_scrollbar_dragged {
				(0.24, 1.0, palette.primary.base.color, 0.5)
			} else {
				idle
			}
		}
	};

	let rail = iced::widget::scrollable::Rail {
		background: Some(palette.background.weak.color.scale_alpha(rail_alpha).into()),
		border: iced::Border {
			color: palette.background.strong.color.scale_alpha(border_alpha * 0.5),
			width: if border_alpha > 0.0 { 1.0 } else { 0.0 },
			radius: CONTROL_RADIUS.into(),
		},
		scroller: iced::widget::scrollable::Scroller {
			background: handle_color.scale_alpha(handle_alpha).into(),
			border: iced::Border {
				color: palette.background.base.color.scale_alpha(border_alpha),
				width: if border_alpha > 0.0 { 1.0 } else { 0.0 },
				radius: CONTROL_RADIUS.into(),
			},
		},
	};
	style.vertical_rail = rail;
	style.gap = Some(palette.background.base.color.into());
	style
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

pub(crate) fn rounded_slider_style(theme: &Theme, status: iced::widget::slider::Status) -> iced::widget::slider::Style {
	let mut style = iced::widget::slider::default(theme, status);
	style.rail.border.radius = CONTROL_RADIUS.into();
	style
}

pub(crate) fn view_sidebar_tab(tab: SidebarTab, is_active: bool) -> Element<'static, Message> {
	let label_text = text(sidebar_tab_label(&tab)).size(14).style(move |theme: &Theme| {
		let palette = theme.palette();
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
				let palette = theme.palette();
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
			.on_press(Message::Sidebar(crate::types::SidebarMessage::SelectTab(tab)))
			.width(Length::Fill)
			.height(Length::Fill)
			.style(move |theme: &Theme, status| {
				let palette = theme.palette();
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
			let palette = theme.palette();
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
