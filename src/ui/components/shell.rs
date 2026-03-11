use iced::widget::{canvas, column, container, row};
use iced::{Element, Length, Size};

use crate::canvas_view::GlyphCanvas;
use crate::editor::EditorViewState;
use crate::scene::LayoutScene;
use crate::types::{CanvasTarget, Message};
use crate::ui::tokens::surface_style;

const STACK_LAYOUT_BREAKPOINT: f32 = 1120.0;

/// Props for the canvas pane.
pub(crate) struct CanvasPaneProps {
	pub(crate) scene: LayoutScene,
	pub(crate) show_baselines: bool,
	pub(crate) show_hitboxes: bool,
	pub(crate) hovered_target: Option<CanvasTarget>,
	pub(crate) selected_target: Option<CanvasTarget>,
	pub(crate) editor: EditorViewState,
	pub(crate) scene_revision: u64,
	pub(crate) stacked: bool,
}

/// Builds the responsive app shell from a sidebar element and a canvas element.
/// This layer decides layout only.
pub(crate) fn view_shell<'a>(
	size: Size, sidebar: Element<'a, Message>, canvas: Element<'a, Message>,
) -> Element<'a, Message> {
	let stacked = size.width < STACK_LAYOUT_BREAKPOINT;
	let content: Element<'a, Message> = if stacked {
		column![canvas, sidebar].spacing(12).into()
	} else {
		row![sidebar, canvas].spacing(16).into()
	};

	container(content)
		.padding(16)
		.width(Length::Fill)
		.height(Length::Fill)
		.into()
}

/// Renders the canvas pane inside the shared app surface.
pub(crate) fn view_canvas_pane(props: CanvasPaneProps) -> Element<'static, Message> {
	let canvas_view = canvas(GlyphCanvas {
		scene: props.scene,
		show_baselines: props.show_baselines,
		show_hitboxes: props.show_hitboxes,
		hovered_target: props.hovered_target,
		selected_target: props.selected_target,
		editor: props.editor,
		scene_revision: props.scene_revision,
	})
	.width(Length::Fill)
	.height(Length::Fill);

	container(canvas_view)
		.padding(8)
		.width(Length::Fill)
		.height(if props.stacked {
			Length::FillPortion(3)
		} else {
			Length::Fill
		})
		.style(surface_style)
		.into()
}
