use {
	super::{
		presenter::{AppViewModel, present},
		sidebar_data::SidebarBodyData,
		state::ShellPane,
		store::AppStore,
	},
	crate::{
		types::{Message, ShellMessage},
		ui::{
			CanvasPaneProps, InspectTabProps, SidebarProps, is_stacked_shell, view_canvas_pane, view_controls_tab,
			view_inspect_tab, view_perf_tab, view_sidebar, view_stacked_shell,
		},
	},
	iced::{
		Element, Length,
		widget::{lazy, pane_grid, responsive},
	},
	std::sync::Arc,
};

impl AppStore {
	pub(crate) fn view(&self) -> Element<'_, Message> {
		let render = present(self);

		responsive(move |size| {
			if is_stacked_shell(size) {
				view_stacked_shell(render_sidebar(&render, true), render_canvas(&render, true))
			} else {
				let render = render.clone();
				let grid = pane_grid(&self.state.shell.chrome, move |_, pane, _| {
					let content = match pane {
						ShellPane::Sidebar => render_sidebar(&render, false),
						ShellPane::Canvas => render_canvas(&render, false),
					};

					pane_grid::Content::new(content)
				})
				.width(Length::Fill)
				.height(Length::Fill)
				.spacing(12)
				.min_size(220)
				.on_resize(12, |event| Message::Shell(ShellMessage::PaneResized(event)));

				iced::widget::container(grid)
					.padding(16)
					.width(Length::Fill)
					.height(Length::Fill)
					.into()
			}
		})
		.width(Length::Fill)
		.height(Length::Fill)
		.into()
	}

	#[cfg(test)]
	pub(super) fn view_sidebar_for_test(&self) -> Element<'_, Message> {
		render_sidebar(&present(self), false)
	}
}

fn render_sidebar(render: &AppViewModel, stacked: bool) -> Element<'static, Message> {
	view_sidebar(SidebarProps {
		active_tab: render.sidebar.active_tab,
		editor_mode: render.sidebar.editor_mode,
		editor_bytes: render.sidebar.editor_bytes,
		undo_depth: render.sidebar.undo_depth,
		redo_depth: render.sidebar.redo_depth,
		body: render_sidebar_body(render.sidebar.body.clone()),
		stacked,
	})
}

fn render_canvas(render: &AppViewModel, stacked: bool) -> Element<'static, Message> {
	view_canvas_pane(CanvasPaneProps {
		snapshot: Arc::clone(&render.snapshot),
		layout_width: render.layout_width,
		decorations: render.decorations,
		inspect_overlays: Arc::clone(&render.inspect_overlays),
		inspect_targets_active: render.inspect_targets_active,
		focused: render.focused,
		scroll: render.scroll,
		perf: render.perf.clone(),
		stacked,
	})
}

fn render_sidebar_body(body: SidebarBodyData) -> Element<'static, Message> {
	match body {
		SidebarBodyData::Controls(data) => view_controls_tab(data),
		SidebarBodyData::Inspect(data) => {
			let key = Arc::as_ptr(&data);
			lazy(key, move |_| {
				view_inspect_tab(&InspectTabProps {
					warnings: data.warnings.clone(),
					interaction_details: data.interaction_details.clone(),
				})
			})
			.into()
		}
		SidebarBodyData::Perf(data) => {
			let key = Arc::as_ptr(&data);
			lazy(key, move |_| view_perf_tab(data.as_ref())).into()
		}
	}
}
