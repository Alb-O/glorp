use {
	crate::{
		types::{
			ControlsMessage, FONT_CHOICES, FontChoice, Message, SAMPLE_PRESETS, SHAPING_CHOICES, SamplePreset,
			ShapingChoice, WRAP_CHOICES, WrapChoice, font_choice_label, sample_preset_label, shaping_choice_label,
			wrap_choice_label,
		},
		ui::{
			PICK_LIST_PADDING, control_row, panel_scrollable, panel_style, rounded_checkbox_style,
			rounded_pick_list_menu_style, rounded_pick_list_style, rounded_slider_style,
		},
	},
	iced::{
		Element, Length,
		widget::{checkbox, column, pick_list, slider, text},
	},
};

#[derive(Debug, Clone, Copy)]
/// Props for the controls tab.
///
/// The tab is a pure view over parent-owned state.
pub(crate) struct ControlsTabProps {
	pub(crate) preset: SamplePreset,
	pub(crate) font: FontChoice,
	pub(crate) shaping: ShapingChoice,
	pub(crate) wrapping: WrapChoice,
	pub(crate) font_size: f32,
	pub(crate) line_height: f32,
	pub(crate) show_baselines: bool,
	pub(crate) show_hitboxes: bool,
}

pub(crate) fn view_controls_tab(props: ControlsTabProps) -> Element<'static, Message> {
	panel_scrollable(
		column![
			control_row(
				"Document",
				pick_list(Some(props.preset), SAMPLE_PRESETS, |preset| sample_preset_label(
					*preset
				)
				.to_owned())
				.on_select(|preset| Message::Controls(ControlsMessage::LoadPreset(preset)))
				.padding(PICK_LIST_PADDING)
				.style(rounded_pick_list_style)
				.menu_style(rounded_pick_list_menu_style)
				.width(Length::Fill)
				.into(),
			),
			control_row(
				"Font",
				pick_list(Some(props.font), FONT_CHOICES, |font| font_choice_label(*font)
					.to_owned())
				.on_select(|font| Message::Controls(ControlsMessage::FontSelected(font)))
				.padding(PICK_LIST_PADDING)
				.style(rounded_pick_list_style)
				.menu_style(rounded_pick_list_menu_style)
				.width(Length::Fill)
				.into(),
			),
			control_row(
				"Shaping",
				pick_list(Some(props.shaping), SHAPING_CHOICES, |shaping| shaping_choice_label(
					*shaping
				)
				.to_owned())
				.on_select(|shaping| Message::Controls(ControlsMessage::ShapingSelected(shaping)))
				.padding(PICK_LIST_PADDING)
				.style(rounded_pick_list_style)
				.menu_style(rounded_pick_list_menu_style)
				.width(Length::Fill)
				.into(),
			),
			control_row(
				"Wrap",
				pick_list(Some(props.wrapping), WRAP_CHOICES, |wrapping| wrap_choice_label(
					*wrapping
				)
				.to_owned())
				.on_select(|wrapping| Message::Controls(ControlsMessage::WrappingSelected(wrapping)))
				.padding(PICK_LIST_PADDING)
				.style(rounded_pick_list_style)
				.menu_style(rounded_pick_list_menu_style)
				.width(Length::Fill)
				.into(),
			),
			control_row(
				format!("Size {:.0}", props.font_size),
				slider(10.0..=48.0, props.font_size, |font_size| {
					Message::Controls(ControlsMessage::FontSizeChanged(font_size))
				})
				.style(rounded_slider_style)
				.width(Length::Fill)
				.into(),
			),
			control_row(
				format!("Line {:.0}", props.line_height),
				slider(12.0..=72.0, props.line_height, |line_height| {
					Message::Controls(ControlsMessage::LineHeightChanged(line_height))
				})
				.style(rounded_slider_style)
				.width(Length::Fill)
				.into(),
			),
			checkbox(props.show_baselines)
				.label("Show baselines and line tops")
				.style(rounded_checkbox_style)
				.on_toggle(|show_baselines| Message::Controls(ControlsMessage::ShowBaselinesChanged(show_baselines))),
			checkbox(props.show_hitboxes)
				.label("Show glyph hitboxes")
				.style(rounded_checkbox_style)
				.on_toggle(|show_hitboxes| Message::Controls(ControlsMessage::ShowHitboxesChanged(show_hitboxes))),
			text("Editor input").size(18),
			view_editor_help(),
		]
		.spacing(14),
	)
	.into()
}

fn view_editor_help() -> Element<'static, Message> {
	iced::widget::container(
		text(
			"Wheel or touchpad-scroll to pan the editor surface.\nClick to focus the editor.\nOpen the Perf tab to watch edit and render timings live while you type or scroll.\nNormal: h/j/k/l or arrows move, i inserts before, a inserts after, x deletes.\nInsert: type, Enter/Tab insert text, Backspace/Delete edit, Esc returns to normal mode.\nUndo/redo: Cmd/Ctrl+Z and Cmd/Ctrl+Shift+Z or Cmd/Ctrl+Y."
		)
		.size(14)
		.width(Length::Fill),
	)
	.padding(12)
	.style(panel_style)
	.into()
}
