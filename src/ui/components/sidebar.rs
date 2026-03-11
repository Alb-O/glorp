use iced::widget::{column, container, row, text};
use iced::{Element, Font, Length};

use crate::types::{Message, SidebarTab};
use crate::ui::{surface_style, view_sidebar_tab};

/// Props for the sidebar shell.
///
/// The parent supplies the active tab body.
pub(crate) struct SidebarProps<'a> {
	pub(crate) active_tab: SidebarTab,
	pub(crate) editor_mode: crate::editor::EditorMode,
	pub(crate) editor_bytes: usize,
	pub(crate) body: Element<'a, Message>,
	pub(crate) stacked: bool,
}

pub(crate) fn view_sidebar<'a>(props: SidebarProps<'a>) -> Element<'a, Message> {
	container(
		column![
			text("Glyph Playground").size(28),
			text(
				"Iced + cosmic-text + swash. Edit the source, then inspect the shaped runs, glyph boxes, and vendored outlines."
			)
			.size(15),
			view_sidebar_tabs(props.active_tab),
			view_editor_status(props.editor_mode, props.editor_bytes),
			container(props.body).height(Length::Fill),
		]
		.spacing(12)
		.padding(16),
	)
	.width(Length::Fill)
	.height(if props.stacked {
		Length::FillPortion(2)
	} else {
		Length::Fill
	})
	.style(surface_style)
	.into()
}

fn view_sidebar_tabs(active_tab: SidebarTab) -> Element<'static, Message> {
	row(SidebarTab::ALL
		.into_iter()
		.map(|tab| view_sidebar_tab(tab, tab == active_tab))
		.collect::<Vec<_>>())
	.spacing(2)
	.into()
}

fn view_editor_status(mode: crate::editor::EditorMode, bytes: usize) -> Element<'static, Message> {
	container(
		text(format!("Editor: {mode} mode, {bytes} bytes"))
			.font(Font::MONOSPACE)
			.size(14),
	)
	.padding([0, 2])
	.into()
}
