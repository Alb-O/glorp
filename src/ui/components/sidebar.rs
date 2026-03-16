use {
	crate::{
		types::{Message, SidebarTab},
		ui::{surface_style, view_sidebar_tab},
	},
	iced::{
		Element, Font, Length,
		widget::{column, container, row, text},
	},
};

/// Props for the sidebar shell.
///
/// The parent supplies the active tab body.
pub(crate) struct SidebarProps<'a> {
	pub(crate) active_tab: SidebarTab,
	pub(crate) editor_mode: crate::editor::EditorMode,
	pub(crate) editor_bytes: usize,
	pub(crate) undo_depth: usize,
	pub(crate) redo_depth: usize,
	pub(crate) body: Element<'a, Message>,
	pub(crate) stacked: bool,
}

pub(crate) fn view_sidebar(props: SidebarProps<'_>) -> Element<'_, Message> {
	container(
		column![
			text("glorp editor").size(28),
			text("Edit the document first. Use Inspect and Perf to inspect shaping, glyph boxes, and runtime cost.")
				.size(15),
			view_sidebar_tabs(props.active_tab),
			view_editor_status(
				props.editor_mode,
				props.editor_bytes,
				props.undo_depth,
				props.redo_depth
			),
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
		.map(|tab| view_sidebar_tab(tab, tab == active_tab)))
	.spacing(2)
	.into()
}

fn view_editor_status(
	mode: crate::editor::EditorMode, bytes: usize, undo_depth: usize, redo_depth: usize,
) -> Element<'static, Message> {
	container(
		text(format!(
			"Editor: {mode} mode, {bytes} bytes, undo {undo_depth}, redo {redo_depth}"
		))
		.font(Font::MONOSPACE)
		.size(14),
	)
	.padding([0, 2])
	.into()
}
