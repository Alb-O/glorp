use {
	super::{Playground, state::ShellPane},
	crate::{
		types::{Message, ShellMessage, SidebarTab},
		ui::{
			CanvasPaneProps, ControlsTabProps, InspectTabProps, PerfTabProps, SidebarProps, is_stacked_shell,
			view_canvas_pane, view_controls_tab, view_inspect_tab, view_perf_tab, view_sidebar, view_stacked_shell,
		},
	},
	iced::{
		Element, Length,
		widget::{pane_grid, responsive},
	},
	std::{fmt::Write as _, sync::Arc},
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
			body: self.view_sidebar_body(),
			stacked,
		})
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
			show_baselines: self.controls.show_baselines,
			show_hitboxes: self.controls.show_hitboxes,
			inspect_overlays,
			editor: self.session.view_state(),
			focused: self.viewport.canvas_focused,
			scene_revision: self.viewport.scene_revision,
			scroll: self.viewport.canvas_scroll,
			perf: self.perf.sink(),
			stacked,
		})
	}

	fn view_sidebar_body(&self) -> Element<'_, Message> {
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
			SidebarTab::Inspect => view_inspect_tab(InspectTabProps {
				warnings: &self.session.scene().warnings,
				interaction_details: self.interaction_details(),
			}),
			SidebarTab::Perf => view_perf_tab(PerfTabProps {
				dashboard: self
					.perf
					.dashboard(self.session.scene(), self.session.mode(), self.session.text().len()),
			}),
		}
	}

	fn interaction_details(&self) -> String {
		let mut details = String::new();
		let _ = writeln!(details, "editor");
		let _ = writeln!(details, "{}", self.session.selection_details());
		let _ = writeln!(details);
		let _ = writeln!(details, "hover");
		let _ = writeln!(
			details,
			"{}",
			self.session
				.scene()
				.target_details(self.sidebar.hovered_target)
				.unwrap_or_else(|| "  none".to_string())
		);
		let _ = writeln!(details);
		let _ = writeln!(details, "selection");
		let _ = writeln!(
			details,
			"{}",
			self.session
				.scene()
				.target_details(self.sidebar.selected_target)
				.unwrap_or_else(|| "  none".to_string())
		);
		details
	}
}
