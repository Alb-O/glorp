use iced::widget::{checkbox, column, pick_list, slider, text};
use iced::{Element, Length};

use crate::types::{ControlsMessage, FontChoice, Message, RenderMode, SamplePreset, ShapingChoice, WrapChoice};
use crate::ui::{
	control_row, panel_scrollable, panel_style, rounded_checkbox_style, rounded_pick_list_menu_style,
	rounded_pick_list_style, rounded_slider_style,
};

/// Props for the controls tab.
///
/// The tab is a pure view over parent-owned state.
pub(crate) struct ControlsTabProps {
	pub(crate) preset: SamplePreset,
	pub(crate) font: FontChoice,
	pub(crate) shaping: ShapingChoice,
	pub(crate) wrapping: WrapChoice,
	pub(crate) render_mode: RenderMode,
	pub(crate) font_size: f32,
	pub(crate) line_height: f32,
	pub(crate) show_baselines: bool,
	pub(crate) show_hitboxes: bool,
}

pub(crate) fn view_controls_tab(props: ControlsTabProps) -> Element<'static, Message> {
	panel_scrollable(
		column![
			control_row(
				"Preset",
				pick_list(SamplePreset::ALL, Some(props.preset), |preset| {
					Message::Controls(ControlsMessage::LoadPreset(preset))
				})
				.style(rounded_pick_list_style)
				.menu_style(rounded_pick_list_menu_style)
				.width(Length::Fill)
				.into(),
			),
			control_row(
				"Font",
				pick_list(FontChoice::ALL, Some(props.font), |font| {
					Message::Controls(ControlsMessage::FontSelected(font))
				})
				.style(rounded_pick_list_style)
				.menu_style(rounded_pick_list_menu_style)
				.width(Length::Fill)
				.into(),
			),
			control_row(
				"Shaping",
				pick_list(ShapingChoice::ALL, Some(props.shaping), |shaping| {
					Message::Controls(ControlsMessage::ShapingSelected(shaping))
				})
				.style(rounded_pick_list_style)
				.menu_style(rounded_pick_list_menu_style)
				.width(Length::Fill)
				.into(),
			),
			control_row(
				"Wrap",
				pick_list(WrapChoice::ALL, Some(props.wrapping), |wrapping| {
					Message::Controls(ControlsMessage::WrappingSelected(wrapping))
				})
				.style(rounded_pick_list_style)
				.menu_style(rounded_pick_list_menu_style)
				.width(Length::Fill)
				.into(),
			),
			control_row(
				"Render",
				pick_list(RenderMode::ALL, Some(props.render_mode), |render_mode| {
					Message::Controls(ControlsMessage::RenderModeSelected(render_mode))
				})
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
			text("Canvas editor").size(18),
			view_editor_help(),
		]
		.spacing(14),
	)
	.into()
}

fn view_editor_help() -> Element<'static, Message> {
	iced::widget::container(
		text(
			"Wheel or touchpad-scroll to pan the canvas.\nClick the canvas to focus.\nOpen the Perf tab to watch edit and render timings live while you type or scroll.\nNormal: h/j/k/l or arrows move, i inserts before, a inserts after, x deletes.\nInsert: type, Enter/Tab insert text, Backspace/Delete edit, Esc returns to normal mode.\nUndo/redo: Cmd/Ctrl+Z and Cmd/Ctrl+Shift+Z or Cmd/Ctrl+Y."
		)
		.size(14)
		.width(Length::Fill),
	)
	.padding(12)
	.style(panel_style)
	.into()
}
