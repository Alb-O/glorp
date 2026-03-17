use {
	super::{
		AppModel,
		sidebar_cache::InspectSidebarArgs,
		sidebar_data::{InspectSidebarData, SidebarBodyData},
		state::ShellPane,
	},
	crate::{
		overlay::OverlayPrimitive,
		perf::{
			CanvasPerfSink, PerfDashboard, PerfFramePacingSummary, PerfGraphSeries, PerfOverview, PerfRecentActivity,
		},
		presentation::SessionSnapshot,
		types::{Message, ShellMessage, SidebarTab},
		ui::{
			CanvasDecorations, CanvasPaneProps, ControlsTabProps, InspectTabProps, SidebarProps, is_stacked_shell,
			view_canvas_pane, view_controls_tab, view_inspect_tab, view_perf_tab, view_sidebar, view_stacked_shell,
		},
	},
	iced::{
		Element, Length, Vector,
		widget::{lazy, pane_grid, responsive},
	},
	std::sync::Arc,
};

#[derive(Debug, Clone)]
struct SidebarRenderModel {
	active_tab: SidebarTab,
	editor_mode: crate::editor::EditorMode,
	editor_bytes: usize,
	undo_depth: usize,
	redo_depth: usize,
	body: SidebarBodyData,
}

#[derive(Debug, Clone)]
struct AppRenderModel {
	snapshot: Arc<SessionSnapshot>,
	layout_width: f32,
	decorations: CanvasDecorations,
	inspect_overlays: Arc<[OverlayPrimitive]>,
	inspect_targets_active: bool,
	focused: bool,
	scroll: Vector,
	perf: CanvasPerfSink,
	sidebar: SidebarRenderModel,
}

impl AppModel {
	pub(crate) fn view(&self) -> Element<'_, Message> {
		let render = self.render_model();

		responsive(move |size| {
			if is_stacked_shell(size) {
				view_stacked_shell(
					render_sidebar(render.clone(), true),
					render_canvas(render.clone(), true),
				)
			} else {
				let render = render.clone();
				let grid = pane_grid(&self.shell.chrome, move |_, pane, _| {
					let content = match pane {
						ShellPane::Sidebar => render_sidebar(render.clone(), false),
						ShellPane::Canvas => render_canvas(render.clone(), false),
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

	pub(crate) fn headless_view(&self) -> Element<'_, ()> {
		self.view().map(|_| ())
	}

	#[cfg(test)]
	pub(super) fn test_view_sidebar(&self) -> Element<'_, Message> {
		render_sidebar(self.render_model(), false)
	}

	fn render_model(&self) -> AppRenderModel {
		let snapshot = Arc::new(self.session.snapshot().clone());
		let inspect_targets_active = self.sidebar.active_tab == SidebarTab::Inspect;

		AppRenderModel {
			inspect_overlays: self.inspect_overlays(snapshot.as_ref(), inspect_targets_active),
			sidebar: SidebarRenderModel {
				active_tab: self.sidebar.active_tab,
				editor_mode: snapshot.mode(),
				editor_bytes: snapshot.editor_bytes(),
				undo_depth: snapshot.editor.undo_depth,
				redo_depth: snapshot.editor.redo_depth,
				body: self.sidebar_body_data(snapshot.as_ref()),
			},
			snapshot,
			layout_width: self.viewport.layout_width,
			decorations: CanvasDecorations {
				show_baselines: self.controls.show_baselines,
				show_hitboxes: self.controls.show_hitboxes,
			},
			inspect_targets_active,
			focused: self.viewport.canvas_focused,
			scroll: self.viewport.canvas_scroll,
			perf: self.perf.sink(),
		}
	}

	fn sidebar_body_data(&self, snapshot: &SessionSnapshot) -> SidebarBodyData {
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
			SidebarTab::Inspect => snapshot.scene.as_ref().map_or_else(
				|| SidebarBodyData::Inspect(Arc::new(unavailable_inspect_sidebar_data())),
				|scene| {
					SidebarBodyData::Inspect(self.sidebar_cache.inspect_data(InspectSidebarArgs {
						editor: &snapshot.editor,
						scene,
						text: self.session.text(),
						hovered_target: self.sidebar.hovered_target,
						selected_target: self.sidebar.selected_target,
					}))
				},
			),
			SidebarTab::Perf => snapshot.scene.as_ref().map_or_else(
				|| {
					SidebarBodyData::Perf(Arc::new(unavailable_perf_dashboard(
						snapshot.mode(),
						snapshot.editor_bytes(),
						self.viewport.layout_width,
					)))
				},
				|scene| SidebarBodyData::Perf(self.sidebar_cache.perf_dashboard(&snapshot.editor, scene, &self.perf)),
			),
		}
	}

	fn inspect_overlays(&self, snapshot: &SessionSnapshot, active: bool) -> Arc<[OverlayPrimitive]> {
		snapshot.scene.as_ref().filter(|_| active).map_or_else(
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
}

fn render_sidebar(render: AppRenderModel, stacked: bool) -> Element<'static, Message> {
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

fn render_canvas(render: AppRenderModel, stacked: bool) -> Element<'static, Message> {
	view_canvas_pane(CanvasPaneProps {
		snapshot: render.snapshot.clone(),
		layout_width: render.layout_width,
		decorations: render.decorations,
		inspect_overlays: render.inspect_overlays.clone(),
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

fn unavailable_inspect_sidebar_data() -> InspectSidebarData {
	InspectSidebarData {
		warnings: Arc::from([]),
		interaction_details: Arc::<str>::from("derived scene unavailable"),
	}
}

fn unavailable_perf_dashboard(
	editor_mode: crate::editor::EditorMode, editor_bytes: usize, layout_width: f32,
) -> PerfDashboard {
	PerfDashboard {
		overview: PerfOverview {
			editor_mode,
			editor_bytes,
			editor_chars: 0,
			line_count: 0,
			run_count: 0,
			glyph_count: 0,
			cluster_count: 0,
			font_count: 0,
			warning_count: 0,
			scene_width: 0.0,
			scene_height: 0.0,
			layout_width,
		},
		hot_paths: Vec::new(),
		recent_activity: vec![PerfRecentActivity {
			label: "scene",
			recent_ms: Arc::from([]),
		}],
		frame_pacing: PerfFramePacingSummary {
			fps: 0.0,
			last_ms: 0.0,
			avg_ms: 0.0,
			max_ms: 0.0,
			total_draws: 0,
			over_budget: 0,
			severe_jank: 0,
			cache_hits: 0,
			cache_misses: 0,
			recent_ms: Arc::from([]),
		},
		graphs: vec![PerfGraphSeries {
			title: "scene",
			samples_ms: Arc::from([]),
			ceiling_ms: 1.0,
			latest_ms: 0.0,
			avg_ms: 0.0,
			p95_ms: 0.0,
			warning_ms: None,
			severe_ms: None,
		}],
	}
}
