use {
	super::{EditorApp, sidebar_cache::InspectSidebarArgs, sidebar_data::SidebarBodyData, state::ShellPane},
	crate::{
		types::{Message, ShellMessage, SidebarTab},
		ui::{
			CanvasDecorations, CanvasPaneProps, ControlsTabProps, InspectTabProps, SidebarProps, is_stacked_shell,
			view_canvas_pane, view_controls_tab, view_inspect_tab, view_perf_tab, view_sidebar, view_stacked_shell,
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
		let snapshot = self.session.snapshot();
		let body = render_sidebar_body(self.sidebar_body_data());
		view_sidebar(SidebarProps {
			active_tab: self.sidebar.active_tab,
			editor_mode: snapshot.mode(),
			editor_bytes: snapshot.editor_bytes(),
			undo_depth: snapshot.editor.undo_depth,
			redo_depth: snapshot.editor.redo_depth,
			body,
			stacked,
		})
	}

	#[cfg(test)]
	pub(super) fn test_view_sidebar(&self) -> Element<'_, Message> {
		self.view_sidebar(false)
	}

	fn view_canvas(&self, stacked: bool) -> Element<'static, Message> {
		let snapshot = self.session.snapshot().clone();
		let inspect_targets_active = self.sidebar.active_tab == SidebarTab::Inspect;
		let inspect_overlays = self.inspect_overlays(snapshot.scene.as_ref(), inspect_targets_active);

		view_canvas_pane(CanvasPaneProps {
			snapshot,
			layout_width: self.viewport.layout_width,
			decorations: CanvasDecorations {
				show_baselines: self.controls.show_baselines,
				show_hitboxes: self.controls.show_hitboxes,
			},
			inspect_overlays,
			inspect_targets_active,
			focused: self.viewport.canvas_focused,
			scroll: self.viewport.canvas_scroll,
			perf: self.perf.sink(),
			stacked,
		})
	}

	fn sidebar_body_data(&self) -> SidebarBodyData {
		match self.sidebar.active_tab {
			SidebarTab::Controls => SidebarBodyData::Controls(ControlsTabProps {
				preset: self.controls.preset,
				font: self.controls.font,
				shaping: self.controls.shaping,
				wrapping: self.controls.wrapping,
				font_size: self.controls.font_size,
				line_height: self.controls.line_height,
				show_baselines: self.controls.show_baselines,
				show_hitboxes: self.controls.show_hitboxes,
			}),
			SidebarTab::Inspect => self.inspect_sidebar_body_data(),
			SidebarTab::Perf => self.perf_sidebar_body_data(),
		}
	}

	fn inspect_sidebar_body_data(&self) -> SidebarBodyData {
		let snapshot = self.session.snapshot();
		let scene = self.required_derived_scene("inspect");
		SidebarBodyData::Inspect(self.sidebar_cache.inspect_data(InspectSidebarArgs {
			editor: &snapshot.editor,
			scene,
			text: self.session.text(),
			hovered_target: self.sidebar.hovered_target,
			selected_target: self.sidebar.selected_target,
		}))
	}

	fn perf_sidebar_body_data(&self) -> SidebarBodyData {
		let snapshot = self.session.snapshot();
		let scene = self.required_derived_scene("perf");
		SidebarBodyData::Perf(self.sidebar_cache.perf_dashboard(&snapshot.editor, scene, &self.perf))
	}

	fn inspect_overlays(
		&self, scene: Option<&crate::presentation::ScenePresentation>, active: bool,
	) -> Arc<[crate::overlay::OverlayPrimitive]> {
		scene.filter(|_| active).map_or_else(
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
	}

	fn required_derived_scene(&self, tab: &str) -> &crate::presentation::ScenePresentation {
		self.session
			.snapshot()
			.scene
			.as_ref()
			.unwrap_or_else(|| panic!("{tab} view requires a materialized derived scene"))
	}
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
