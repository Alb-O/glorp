use {
	crate::{
		app::{Message, SIDEBAR_TABS, SidebarTab},
		panels::{surface_style, view_sidebar_tab},
	},
	iced::{
		Element, Font, Length,
		widget::{column, container, row, text},
	},
};

/// Props for the sidebar shell.
///
/// The parent supplies the active tab body.
pub struct SidebarProps<'a> {
	pub active_tab: SidebarTab,
	pub editor_mode: glorp_editor::EditorMode,
	pub editor_bytes: usize,
	pub undo_depth: usize,
	pub redo_depth: usize,
	pub body: Element<'a, Message>,
	pub stacked: bool,
}

pub fn view_sidebar(props: SidebarProps<'_>) -> Element<'_, Message> {
	let SidebarProps {
		active_tab,
		editor_mode,
		editor_bytes,
		undo_depth,
		redo_depth,
		body,
		stacked,
	} = props;

	container(
		column![
			text("glorp editor").size(28),
			text("Edit the document first. Use Inspect and Perf to inspect shaping, glyph boxes, and runtime cost.")
				.size(15),
			view_sidebar_tabs(active_tab),
			view_editor_status(editor_mode, editor_bytes, undo_depth, redo_depth),
			container(body).height(Length::Fill),
		]
		.spacing(12)
		.padding(16),
	)
	.width(Length::Fill)
	.height(if stacked { Length::FillPortion(2) } else { Length::Fill })
	.style(surface_style)
	.into()
}

fn view_sidebar_tabs(active_tab: SidebarTab) -> Element<'static, Message> {
	row(SIDEBAR_TABS
		.into_iter()
		.map(|tab| view_sidebar_tab(tab, tab == active_tab)))
	.spacing(2)
	.into()
}

fn view_editor_status(
	mode: glorp_editor::EditorMode, bytes: usize, undo_depth: usize, redo_depth: usize,
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
