use iced::widget::{pane_grid, responsive};
use iced::{Element, Length, Subscription, Task, futures, stream};
use iced::{Size, Vector};

use std::fmt::Write as _;
use std::time::{Duration, Instant};

use crate::canvas_view::scene_viewport_size;
use crate::editor::EditorBuffer;
use crate::perf::PerfMonitor;
use crate::scene::{LayoutSceneModel, make_font_system, scene_config};
use crate::types::{FontChoice, Message, RenderMode, SamplePreset, ShapingChoice, SidebarTab, WrapChoice};
use crate::ui::{
	CanvasPaneProps, ControlsTabProps, InspectTabProps, PerfTabProps, SidebarProps, default_sidebar_ratio,
	is_stacked_shell, view_canvas_pane, view_controls_tab, view_dump_tab, view_inspect_tab, view_perf_tab,
	view_sidebar, view_stacked_shell,
};

const RESIZE_REFLOW_INTERVAL: Duration = Duration::from_millis(16);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShellPane {
	Sidebar,
	Canvas,
}

#[derive(Debug, Clone, Copy)]
struct ResizeCoalescer {
	applied_width: f32,
	pending_width: Option<f32>,
	last_applied_at: Option<Instant>,
}

impl ResizeCoalescer {
	fn new(width: f32) -> Self {
		Self {
			applied_width: width,
			pending_width: None,
			last_applied_at: None,
		}
	}

	fn observe(&mut self, width: f32, now: Instant) -> Option<f32> {
		if (self.applied_width - width).abs() < 0.5 && self.pending_width.is_none() {
			return None;
		}

		self.pending_width = Some(width);

		if self
			.last_applied_at
			.is_none_or(|last| now.duration_since(last) >= RESIZE_REFLOW_INTERVAL)
		{
			return self.flush(now);
		}

		None
	}

	fn flush(&mut self, now: Instant) -> Option<f32> {
		let width = self.pending_width?;
		self.pending_width = None;

		if (self.applied_width - width).abs() < 0.5 {
			return None;
		}

		self.applied_width = width;
		self.last_applied_at = Some(now);
		Some(width)
	}

	fn has_pending(&self) -> bool {
		self.pending_width.is_some()
	}

	fn mark_applied(&mut self, width: f32, now: Instant) {
		self.applied_width = width;
		self.last_applied_at = Some(now);

		if self.pending_width.is_some_and(|pending| (pending - width).abs() < 0.5) {
			self.pending_width = None;
		}
	}
}

pub(crate) struct Playground {
	editor: EditorBuffer,
	preset: SamplePreset,
	font: FontChoice,
	shaping: ShapingChoice,
	wrapping: WrapChoice,
	render_mode: RenderMode,
	font_size: f32,
	line_height: f32,
	layout_width: f32,
	show_baselines: bool,
	show_hitboxes: bool,
	active_sidebar_tab: SidebarTab,
	hovered_target: Option<crate::types::CanvasTarget>,
	selected_target: Option<crate::types::CanvasTarget>,
	canvas_scroll: Vector,
	canvas_viewport: Size,
	scene: LayoutSceneModel,
	scene_dump: String,
	font_system: cosmic_text::FontSystem,
	chrome: pane_grid::State<ShellPane>,
	resize_coalescer: ResizeCoalescer,
	perf: PerfMonitor,
	scene_revision: u64,
}

impl Playground {
	pub(crate) fn new() -> (Self, Task<Message>) {
		let mut font_system = make_font_system();
		let preset = SamplePreset::Tall;
		let font = FontChoice::JetBrainsMono;
		let shaping = ShapingChoice::Advanced;
		let wrapping = WrapChoice::Word;
		let render_mode = RenderMode::CanvasOnly;
		let font_size = 24.0;
		let line_height = 32.0;
		let layout_width = 540.0;
		let show_baselines = false;
		let show_hitboxes = false;
		let active_sidebar_tab = SidebarTab::Controls;
		let perf = PerfMonitor::default();
		let config = scene_config(
			font,
			shaping,
			wrapping,
			render_mode,
			font_size,
			line_height,
			layout_width,
		);
		let chrome = pane_grid::State::with_configuration(pane_grid::Configuration::Split {
			axis: pane_grid::Axis::Vertical,
			ratio: default_sidebar_ratio(),
			a: Box::new(pane_grid::Configuration::Pane(ShellPane::Sidebar)),
			b: Box::new(pane_grid::Configuration::Pane(ShellPane::Canvas)),
		});
		let editor = EditorBuffer::new(&mut font_system, preset.text(), config);
		let scene = LayoutSceneModel::new(&mut font_system, editor.text(), editor.buffer(), config);

		(
			Self {
				editor,
				preset,
				font,
				shaping,
				wrapping,
				render_mode,
				font_size,
				line_height,
				layout_width,
				show_baselines,
				show_hitboxes,
				active_sidebar_tab,
				hovered_target: None,
				selected_target: None,
				canvas_scroll: Vector::ZERO,
				canvas_viewport: Size::new(layout_width, 320.0),
				scene,
				scene_dump: String::new(),
				font_system,
				chrome,
				resize_coalescer: ResizeCoalescer::new(layout_width),
				perf,
				scene_revision: 1,
			},
			Task::none(),
		)
	}

	pub(crate) fn subscription(&self) -> Subscription<Message> {
		let mut subscriptions = Vec::new();

		if self.active_sidebar_tab == SidebarTab::Perf {
			subscriptions.push(Subscription::run(perf_tick_stream).map(Message::PerfTick));
		}

		if self.resize_coalescer.has_pending() {
			subscriptions.push(Subscription::run(resize_tick_stream).map(Message::ResizeTick));
		}

		Subscription::batch(subscriptions)
	}

	pub(crate) fn update(&mut self, message: Message) -> Task<Message> {
		match message {
			Message::LoadPreset(preset) => {
				self.preset = preset;
				if !matches!(preset, SamplePreset::Custom) {
					let config = self.current_scene_config();
					self.editor.reset(&mut self.font_system, preset.text(), config);
					self.refresh_scene();
				}
			}
			Message::FontSelected(font) => {
				self.font = font;
				self.refresh_scene();
			}
			Message::ShapingSelected(shaping) => {
				self.shaping = shaping;
				self.refresh_scene();
			}
			Message::WrappingSelected(wrapping) => {
				self.wrapping = wrapping;
				self.refresh_scene();
			}
			Message::RenderModeSelected(render_mode) => {
				self.render_mode = render_mode;
				self.refresh_scene();
			}
			Message::FontSizeChanged(font_size) => {
				self.font_size = font_size;
				self.line_height = self.line_height.max(self.font_size);
				self.refresh_scene();
			}
			Message::LineHeightChanged(line_height) => {
				self.line_height = line_height;
				self.refresh_scene();
			}
			Message::CanvasViewportResized(size) => {
				let viewport = scene_viewport_size(size);
				let layout_width = viewport.width;
				let now = Instant::now();
				self.canvas_viewport = viewport;
				self.apply_live_layout_width(layout_width);

				if let Some(layout_width) = self.resize_coalescer.observe(layout_width, now) {
					self.apply_resize_scene_width(layout_width);
				}
			}
			Message::ResizeTick(now) => {
				if let Some(layout_width) = self.resize_coalescer.flush(now) {
					self.apply_resize_scene_width(layout_width);
				}
			}
			Message::ShowBaselinesChanged(show_baselines) => {
				self.show_baselines = show_baselines;
				self.scene_revision += 1;
			}
			Message::ShowHitboxesChanged(show_hitboxes) => {
				self.show_hitboxes = show_hitboxes;
				self.scene_revision += 1;
			}
			Message::SelectSidebarTab(tab) => {
				self.active_sidebar_tab = tab;
				if tab != SidebarTab::Inspect {
					self.hovered_target = None;
					self.selected_target = None;
				}
				if matches!(tab, SidebarTab::Dump) {
					self.refresh_scene_dump();
				}
			}
			Message::PerfTick(_now) => {}
			Message::CanvasHovered(target) => {
				self.hovered_target = (self.active_sidebar_tab == SidebarTab::Inspect)
					.then_some(target)
					.flatten();
			}
			Message::CanvasScrollChanged(scroll) => {
				self.canvas_scroll = scroll;
			}
			Message::CanvasPressed {
				target,
				position,
				double_click,
			} => {
				self.selected_target = (self.active_sidebar_tab == SidebarTab::Inspect)
					.then_some(target)
					.flatten();
				self.apply_editor_command(
					crate::editor::EditorCommand::BeginPointerSelection {
						position,
						select_word: double_click,
					},
					false,
					true,
				);
			}
			Message::CanvasDragged(position) => {
				self.apply_editor_command(
					crate::editor::EditorCommand::DragPointerSelection(position),
					false,
					false,
				);
			}
			Message::CanvasReleased => {
				self.apply_editor_command(crate::editor::EditorCommand::EndPointerSelection, false, false);
			}
			Message::PaneResized(event) => {
				self.chrome.resize(event.split, event.ratio);
			}
			Message::EditorCommand(command) => {
				self.apply_editor_command(command, true, true);
			}
		}

		self.perf.flush_canvas_metrics();
		Task::none()
	}

	pub(crate) fn view(&self) -> Element<'_, Message> {
		responsive(|size| {
			if is_stacked_shell(size) {
				let sidebar = self.view_sidebar(true);
				let canvas = self.view_canvas(true);
				return view_stacked_shell(sidebar, canvas);
			}

			let grid = pane_grid(&self.chrome, |_, pane, _| {
				pane_grid::Content::new(match pane {
					ShellPane::Sidebar => self.view_sidebar(false),
					ShellPane::Canvas => self.view_canvas(false),
				})
			})
			.width(Length::Fill)
			.height(Length::Fill)
			.spacing(12)
			.min_size(220)
			.on_resize(12, Message::PaneResized);

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

	fn view_sidebar(&self, stacked: bool) -> Element<'_, Message> {
		let (undo_depth, redo_depth) = self.editor.history_depths();
		view_sidebar(SidebarProps {
			active_tab: self.active_sidebar_tab,
			editor_mode: self.editor.mode(),
			editor_bytes: self.editor.text().len(),
			undo_depth,
			redo_depth,
			body: self.view_sidebar_body(),
			stacked,
		})
	}

	fn view_canvas(&self, stacked: bool) -> Element<'static, Message> {
		view_canvas_pane(CanvasPaneProps {
			scene: self.scene.scene().clone(),
			layout_width: self.layout_width,
			show_inspector_overlays: self.active_sidebar_tab == SidebarTab::Inspect,
			show_baselines: self.show_baselines,
			show_hitboxes: self.show_hitboxes,
			hovered_target: self.hovered_target,
			selected_target: self.selected_target,
			editor: self.editor.view_state(),
			scene_revision: self.scene_revision,
			scroll: self.canvas_scroll,
			perf: self.perf.bridge(),
			stacked,
		})
	}

	fn view_sidebar_body(&self) -> Element<'_, Message> {
		match self.active_sidebar_tab {
			SidebarTab::Controls => view_controls_tab(ControlsTabProps {
				preset: self.preset,
				font: self.font,
				shaping: self.shaping,
				wrapping: self.wrapping,
				render_mode: self.render_mode,
				font_size: self.font_size,
				line_height: self.line_height,
				show_baselines: self.show_baselines,
				show_hitboxes: self.show_hitboxes,
			}),
			SidebarTab::Inspect => view_inspect_tab(InspectTabProps {
				warnings: &self.scene.scene().warnings,
				interaction_details: self.interaction_details(),
			}),
			SidebarTab::Dump => view_dump_tab(&self.scene_dump),
			SidebarTab::Perf => view_perf_tab(PerfTabProps {
				overview: self
					.perf
					.overview_text(self.scene.scene(), self.editor.mode(), self.editor.text().len()),
				graphs: self.perf.graphs(),
				frame_pacing: self.perf.frame_pacing_text(),
				hot_paths: self.perf.hot_paths_text(),
				recent_activity: self.perf.recent_activity_text(),
			}),
		}
	}

	fn refresh_scene(&mut self) {
		let duration = self.rebuild_scene();
		self.finish_scene_refresh(true);
		self.perf.record_scene_build(duration);
	}

	fn refresh_scene_dump(&mut self) {
		self.scene_dump = self.scene.scene().dump_text();
	}

	fn apply_editor_command(
		&mut self, command: crate::editor::EditorCommand, mark_custom: bool, reveal_viewport: bool,
	) {
		let command_started = Instant::now();
		let apply_started = Instant::now();
		let result = self.editor.apply(&mut self.font_system, command);
		self.perf.record_editor_apply(apply_started.elapsed());

		if mark_custom {
			self.preset = SamplePreset::Custom;
		}

		if result.changed {
			self.refresh_scene_after_edit();
		}
		if reveal_viewport {
			self.reveal_editor_target();
		}

		self.perf.record_editor_command(command_started.elapsed());
	}

	fn interaction_details(&self) -> String {
		let mut details = String::new();
		let _ = writeln!(details, "editor");
		let _ = writeln!(details, "{}", self.editor.selection_details());
		let _ = writeln!(details);
		let _ = writeln!(details, "hover");
		let _ = writeln!(
			details,
			"{}",
			self.scene
				.scene()
				.target_details(self.hovered_target)
				.unwrap_or_else(|| "  none".to_string())
		);
		let _ = writeln!(details);
		let _ = writeln!(details, "selection");
		let _ = writeln!(
			details,
			"{}",
			self.scene
				.scene()
				.target_details(self.selected_target)
				.unwrap_or_else(|| "  none".to_string())
		);
		details
	}

	fn refresh_scene_after_edit(&mut self) {
		let duration = self.rebuild_scene();
		self.finish_scene_refresh(false);
		self.perf.record_scene_build(duration);
	}

	fn current_scene_config(&self) -> crate::scene::SceneConfig {
		scene_config(
			self.font,
			self.shaping,
			self.wrapping,
			self.render_mode,
			self.font_size,
			self.line_height,
			self.layout_width,
		)
	}

	fn apply_live_layout_width(&mut self, layout_width: f32) {
		if (self.layout_width - layout_width).abs() < 0.5 {
			return;
		}

		self.layout_width = layout_width;
		self.editor.sync_buffer_width(&mut self.font_system, layout_width);
	}

	fn apply_resize_scene_width(&mut self, layout_width: f32) {
		self.layout_width = layout_width;
		let duration = self.rebuild_scene();
		self.finish_scene_refresh(false);
		self.perf.record_scene_build(duration);
		self.perf.record_resize_reflow(duration);
	}

	fn rebuild_scene(&mut self) -> Duration {
		let started = Instant::now();
		let config = self.current_scene_config();
		self.editor.sync_buffer_config(&mut self.font_system, config);
		self.scene
			.rebuild(&mut self.font_system, self.editor.text(), self.editor.buffer(), config);
		self.resize_coalescer.mark_applied(config.max_width, Instant::now());
		started.elapsed()
	}

	fn finish_scene_refresh(&mut self, reset_scroll: bool) {
		self.hovered_target = None;
		if reset_scroll {
			self.canvas_scroll = Vector::ZERO;
		}
		if matches!(self.active_sidebar_tab, SidebarTab::Dump) {
			self.refresh_scene_dump();
		} else {
			self.scene_dump.clear();
		}
		self.scene_revision += 1;
	}

	fn reveal_editor_target(&mut self) {
		let Some(target) = self.editor.view_state().viewport_target else {
			self.canvas_scroll = self.clamp_scroll(self.canvas_scroll);
			return;
		};

		let viewport = self.canvas_viewport;
		let mut scroll = self.clamp_scroll(self.canvas_scroll);
		let margin_x = 24.0;
		let margin_y = 24.0;
		let left = target.x;
		let right = target.x + target.width.max(1.0);
		let top = target.y;
		let bottom = target.y + target.height.max(1.0);

		if left < scroll.x + margin_x {
			scroll.x = (left - margin_x).max(0.0);
		} else if right > scroll.x + viewport.width - margin_x {
			scroll.x = (right - viewport.width + margin_x).max(0.0);
		}

		if top < scroll.y + margin_y {
			scroll.y = (top - margin_y).max(0.0);
		} else if bottom > scroll.y + viewport.height - margin_y {
			scroll.y = (bottom - viewport.height + margin_y).max(0.0);
		}

		self.canvas_scroll = self.clamp_scroll(scroll);
	}

	fn clamp_scroll(&self, scroll: Vector) -> Vector {
		let max_x = if matches!(self.scene.scene().wrapping, WrapChoice::None) {
			(self.scene.scene().measured_width.max(self.layout_width) - self.canvas_viewport.width).max(0.0)
		} else {
			(self.layout_width - self.canvas_viewport.width).max(0.0)
		};
		let max_y = (self.scene.scene().measured_height - self.canvas_viewport.height).max(0.0);

		Vector::new(scroll.x.clamp(0.0, max_x), scroll.y.clamp(0.0, max_y))
	}
}

#[cfg(test)]
mod tests {
	use super::{Playground, RESIZE_REFLOW_INTERVAL, ResizeCoalescer};
	use crate::editor::EditorCommand;
	use crate::types::Message;
	use iced::{Size, Vector};
	use std::time::{Duration, Instant};

	#[test]
	fn resize_coalescer_limits_burst_reflows_and_flushes_latest_width() {
		let started = Instant::now();
		let mut coalescer = ResizeCoalescer::new(600.0);

		assert_eq!(coalescer.observe(700.0, started), Some(700.0));
		assert_eq!(coalescer.observe(710.0, started + Duration::from_millis(4)), None);
		assert_eq!(coalescer.observe(720.0, started + Duration::from_millis(8)), None);
		assert!(coalescer.has_pending());
		assert_eq!(coalescer.flush(started + RESIZE_REFLOW_INTERVAL), Some(720.0));
		assert!(!coalescer.has_pending());
	}

	#[test]
	fn edits_preserve_visible_scroll_position() {
		let (mut playground, _) = Playground::new();
		let _ = playground.update(Message::CanvasViewportResized(Size::new(760.0, 280.0)));

		for _ in 0..5 {
			let _ = playground.update(Message::EditorCommand(EditorCommand::MoveDown));
		}

		let target = playground
			.editor
			.view_state()
			.viewport_target
			.expect("selection should expose a viewport target");
		playground.canvas_scroll = Vector::new(0.0, (target.y - 40.0).max(0.0));
		let previous_scroll = playground.canvas_scroll;

		let _ = playground.update(Message::EditorCommand(EditorCommand::EnterInsertAfter));
		let _ = playground.update(Message::EditorCommand(EditorCommand::InsertText("!".to_string())));

		assert_eq!(playground.canvas_scroll, previous_scroll);
	}

	#[test]
	fn keyboard_motion_reveals_caret_when_it_leaves_viewport() {
		let (mut playground, _) = Playground::new();
		let _ = playground.update(Message::CanvasViewportResized(Size::new(760.0, 220.0)));

		for _ in 0..12 {
			let _ = playground.update(Message::EditorCommand(EditorCommand::MoveDown));
		}

		assert!(playground.canvas_scroll.y > 0.0);
	}
}

fn perf_tick_stream() -> impl futures::Stream<Item = iced::time::Instant> {
	tick_stream(Duration::from_millis(100))
}

fn resize_tick_stream() -> impl futures::Stream<Item = iced::time::Instant> {
	tick_stream(RESIZE_REFLOW_INTERVAL)
}

fn tick_stream(interval: Duration) -> impl futures::Stream<Item = iced::time::Instant> {
	stream::channel(1, async move |mut output| {
		use futures::SinkExt;

		loop {
			std::thread::sleep(interval);

			if output.send(iced::time::Instant::now()).await.is_err() {
				break;
			}
		}
	})
}
