use {
	super::{
		Playground,
		sidebar_cache::{InspectSidebarModel, PerfSidebarModel},
		state::ShellPane,
	},
	crate::{
		types::{Message, ShellMessage, SidebarTab},
		ui::{
			CanvasDecorations, CanvasPaneProps, ControlsTabProps, SidebarProps, is_stacked_shell, view_canvas_pane,
			view_controls_tab, view_inspect_tab, view_perf_tab, view_sidebar, view_stacked_shell,
		},
	},
	iced::{
		Element, Length,
		widget::{lazy, pane_grid, responsive},
	},
	std::sync::Arc,
};

impl Playground {
	pub(crate) fn view(&self) -> Element<'_, Message> {
		responsive(|size| {
			if is_stacked_shell(size) {
				let sidebar = self.view_sidebar(true);
				let canvas = self.view_canvas(true);
				return view_stacked_shell(sidebar, canvas);
			}

			let grid = pane_grid(&self.shell.chrome, |_, pane, _| {
				pane_grid::Content::new(match pane {
					ShellPane::Sidebar => self.view_sidebar(false),
					ShellPane::Canvas => self.view_canvas(false),
				})
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
		})
		.width(Length::Fill)
		.height(Length::Fill)
		.into()
	}

	pub fn headless_view(&self) -> Element<'_, ()> {
		self.view().map(|_| ())
	}

	fn view_sidebar(&self, stacked: bool) -> Element<'_, Message> {
		let (undo_depth, redo_depth) = self.session.history_depths();
		view_sidebar(SidebarProps {
			active_tab: self.sidebar.active_tab,
			editor_mode: self.session.mode(),
			editor_bytes: self.session.text().len(),
			undo_depth,
			redo_depth,
			body: self.view_sidebar_body(undo_depth, redo_depth),
			stacked,
		})
	}

	#[cfg(test)]
	pub(super) fn test_view_sidebar(&self) -> Element<'_, Message> {
		self.view_sidebar(false)
	}

	fn view_canvas(&self, stacked: bool) -> Element<'static, Message> {
		let inspect_overlays = if self.sidebar.active_tab == SidebarTab::Inspect {
			self.session.inspect_overlay_primitives(
				self.sidebar.hovered_target,
				self.sidebar.selected_target,
				self.viewport.layout_width,
				self.controls.show_hitboxes,
			)
		} else {
			Arc::from([])
		};

		view_canvas_pane(CanvasPaneProps {
			scene: self.session.scene().clone(),
			text_layer: self.session.text_layer_state(),
			layout_width: self.viewport.layout_width,
			decorations: CanvasDecorations {
				show_baselines: self.controls.show_baselines,
				show_hitboxes: self.controls.show_hitboxes,
			},
			inspect_overlays,
			editor: self.session.view_state(),
			focused: self.viewport.canvas_focused,
			scene_revision: self.viewport.scene_revision,
			scroll: self.viewport.canvas_scroll,
			perf: self.perf.sink(),
			stacked,
		})
	}

	fn view_sidebar_body(&self, undo_depth: usize, redo_depth: usize) -> Element<'static, Message> {
		match self.sidebar.active_tab {
			SidebarTab::Controls => view_controls_tab(ControlsTabProps {
				preset: self.controls.preset,
				font: self.controls.font,
				shaping: self.controls.shaping,
				wrapping: self.controls.wrapping,
				render_mode: self.controls.render_mode,
				font_size: self.controls.font_size,
				line_height: self.controls.line_height,
				show_baselines: self.controls.show_baselines,
				show_hitboxes: self.controls.show_hitboxes,
			}),
			SidebarTab::Inspect => self.view_inspect_sidebar(undo_depth, redo_depth),
			SidebarTab::Perf => self.view_perf_sidebar(),
		}
	}

	fn view_inspect_sidebar(&self, undo_depth: usize, redo_depth: usize) -> Element<'static, Message> {
		let editor = self.session.view_state();
		let model = self.sidebar_cache.inspect_model(
			self.viewport.scene_revision,
			self.session.scene(),
			&editor,
			self.sidebar.hovered_target,
			self.sidebar.selected_target,
			undo_depth,
			redo_depth,
		);

		lazy(model.key, move |_| {
			let InspectSidebarModel { props, .. } = model.clone();
			view_inspect_tab((*props).clone())
		})
		.into()
	}

	fn view_perf_sidebar(&self) -> Element<'static, Message> {
		let editor_mode = self.session.mode();
		let editor_bytes = self.session.text().len();
		let model = self.sidebar_cache.perf_model(
			self.viewport.scene_revision,
			self.session.scene(),
			&self.perf,
			editor_mode,
			editor_bytes,
		);

		lazy(model.key, move |_| {
			let PerfSidebarModel { props, .. } = model.clone();
			view_perf_tab((*props).clone())
		})
		.into()
	}
}
