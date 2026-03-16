use {
	super::{
		EditorApp,
		sidebar_cache::InspectSidebarArgs,
		sidebar_data::{ControlsSidebarData, SidebarBodyData},
		state::ShellPane,
	},
	crate::{
		types::{Message, ShellMessage, SidebarTab},
		ui::{
			CanvasDecorations, CanvasPaneProps, ControlsTabProps, InspectTabProps, PerfTabProps, SidebarProps,
			is_stacked_shell, view_canvas_pane, view_controls_tab, view_inspect_tab, view_perf_tab, view_sidebar,
			view_stacked_shell,
		},
	},
	iced::{
		Element, Length,
		widget::{lazy, pane_grid, responsive},
	},
	std::sync::Arc,
};

impl EditorApp {
	pub(crate) fn view(&self) -> Element<'_, Message> {
		responsive(|size| {
			if is_stacked_shell(size) {
				view_stacked_shell(self.view_sidebar(true), self.view_canvas(true))
			} else {
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
			}
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
		let presentation = self.session.editor_presentation();
		let body = render_sidebar_body(self.sidebar_body_data(undo_depth, redo_depth));
		view_sidebar(SidebarProps {
			active_tab: self.sidebar.active_tab,
			editor_mode: presentation.mode(),
			editor_bytes: presentation.editor_bytes(),
			undo_depth,
			redo_depth,
			body,
			stacked,
		})
	}

	#[cfg(test)]
	pub(super) fn test_view_sidebar(&self) -> Element<'_, Message> {
		self.view_sidebar(false)
	}

	fn view_canvas(&self, stacked: bool) -> Element<'static, Message> {
		let editor_presentation = self.session.editor_presentation();
		let derived_scene = self.session.derived_scene().cloned();
		let inspect_overlays = if self.sidebar.active_tab == SidebarTab::Inspect {
			derived_scene.as_ref().map_or_else(
				|| Arc::from([]),
				|scene| {
					scene.layout.inspect_overlay_primitives(
						self.sidebar.hovered_target,
						self.sidebar.selected_target,
						self.viewport.layout_width,
						self.controls.show_hitboxes,
					)
				},
			)
		} else {
			Arc::from([])
		};

		view_canvas_pane(CanvasPaneProps {
			editor_presentation: editor_presentation.clone(),
			derived_scene,
			layout_width: self.viewport.layout_width,
			decorations: CanvasDecorations {
				show_baselines: self.controls.show_baselines,
				show_hitboxes: self.controls.show_hitboxes,
			},
			inspect_overlays,
			inspect_targets_active: self.sidebar.active_tab == SidebarTab::Inspect,
			focused: self.viewport.canvas_focused,
			scene_revision: self.viewport.scene_revision,
			scroll: self.viewport.canvas_scroll,
			perf: self.perf.sink(),
			stacked,
		})
	}

	fn sidebar_body_data(&self, undo_depth: usize, redo_depth: usize) -> SidebarBodyData {
		match self.sidebar.active_tab {
			SidebarTab::Controls => SidebarBodyData::Controls(ControlsSidebarData {
				preset: self.controls.preset,
				font: self.controls.font,
				shaping: self.controls.shaping,
				wrapping: self.controls.wrapping,
				font_size: self.controls.font_size,
				line_height: self.controls.line_height,
				show_baselines: self.controls.show_baselines,
				show_hitboxes: self.controls.show_hitboxes,
			}),
			SidebarTab::Inspect => self.inspect_sidebar_body_data(undo_depth, redo_depth),
			SidebarTab::Perf => self.perf_sidebar_body_data(),
		}
	}

	fn inspect_sidebar_body_data(&self, undo_depth: usize, redo_depth: usize) -> SidebarBodyData {
		let scene = self
			.session
			.derived_scene()
			.expect("inspect view requires a materialized derived scene");
		SidebarBodyData::Inspect(
			self.sidebar_cache
				.inspect_model(InspectSidebarArgs {
					editor: self.session.editor_presentation(),
					scene,
					text: self.session.text(),
					hovered_target: self.sidebar.hovered_target,
					selected_target: self.sidebar.selected_target,
					undo_depth,
					redo_depth,
				})
				.data,
		)
	}

	fn perf_sidebar_body_data(&self) -> SidebarBodyData {
		let scene = self
			.session
			.derived_scene()
			.expect("perf view requires a materialized derived scene");
		SidebarBodyData::Perf(
			self.sidebar_cache
				.perf_model(self.session.editor_presentation(), scene, &self.perf)
				.data,
		)
	}
}

fn render_sidebar_body(body: SidebarBodyData) -> Element<'static, Message> {
	match body {
		SidebarBodyData::Controls(data) => view_controls_tab(ControlsTabProps {
			preset: data.preset,
			font: data.font,
			shaping: data.shaping,
			wrapping: data.wrapping,
			font_size: data.font_size,
			line_height: data.line_height,
			show_baselines: data.show_baselines,
			show_hitboxes: data.show_hitboxes,
		}),
		SidebarBodyData::Inspect(data) => {
			let key = Arc::as_ptr(&data);
			lazy(key, move |_| {
				let props = InspectTabProps {
					warnings: data.warnings.clone(),
					interaction_details: data.interaction_details.clone(),
				};
				view_inspect_tab(&props)
			})
			.into()
		}
		SidebarBodyData::Perf(data) => {
			let key = Arc::as_ptr(&data);
			lazy(key, move |_| {
				view_perf_tab(PerfTabProps {
					dashboard: data.dashboard.clone(),
				})
			})
			.into()
		}
	}
}
