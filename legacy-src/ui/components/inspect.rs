use {
	crate::{
		types::Message,
		ui::{CONTROL_RADIUS, panel_scrollable, panel_style},
	},
	iced::{
		Element, Font, Length, Theme,
		widget::{column, container, text},
	},
	std::sync::Arc,
};

/// Props for the inspect tab.
pub(crate) struct InspectTabProps {
	pub(crate) warnings: Arc<[String]>,
	pub(crate) interaction_details: Arc<str>,
}

pub(crate) fn view_inspect_tab(props: &InspectTabProps) -> Element<'static, Message> {
	panel_scrollable(
		column![
			text("Warnings").size(18),
			view_warnings_panel(&props.warnings),
			text("Hover and selection").size(18),
			view_interaction_panel(&props.interaction_details),
		]
		.spacing(12),
	)
	.into()
}

fn view_warnings_panel(warnings: &[String]) -> Element<'static, Message> {
	let has_warnings = !warnings.is_empty();
	let warnings_text = if has_warnings {
		warnings.join("\n")
	} else {
		"No warnings".into()
	};

	container(text(warnings_text).size(14).width(Length::Fill))
		.padding(12)
		.style(move |theme: &Theme| {
			let palette = theme.palette();
			let (background, border) = if has_warnings {
				(palette.warning.weak.color, palette.warning.strong.color)
			} else {
				(palette.background.weak.color, palette.background.strong.color)
			};
			container::Style {
				background: Some(background.into()),
				border: iced::Border {
					color: border,
					width: 1.0,
					radius: CONTROL_RADIUS.into(),
				},
				..Default::default()
			}
		})
		.into()
}

fn view_interaction_panel(interaction_details: &Arc<str>) -> Element<'static, Message> {
	container(
		panel_scrollable(
			text(interaction_details.as_ref().to_owned())
				.font(Font::MONOSPACE)
				.size(14)
				.width(Length::Fill),
		)
		.height(Length::Shrink),
	)
	.padding(12)
	.style(panel_style)
	.into()
}
