use iced::{Size, Subscription, Task, futures, stream};

use std::time::{Duration, Instant};

use crate::editor::EditorCommand;
use crate::types::{Message, SamplePreset, SidebarTab};

use super::Playground;
use super::state::{EditorDispatchSource, RESIZE_REFLOW_INTERVAL};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SceneRefreshReason {
	PresetLoaded,
	ControlsChanged,
	DocumentEdited,
	ResizeReflow,
}

impl SceneRefreshReason {
	fn resets_scroll(self) -> bool {
		matches!(self, Self::PresetLoaded | Self::ControlsChanged)
	}

	fn records_resize_reflow(self) -> bool {
		matches!(self, Self::ResizeReflow)
	}
}

impl Playground {
	pub(crate) fn subscription(&self) -> Subscription<Message> {
		let mut subscriptions = Vec::new();

		if self.sidebar.active_tab == SidebarTab::Perf {
			subscriptions.push(Subscription::run(perf_tick_stream).map(Message::PerfTick));
		}

		if self.viewport.resize_coalescer.has_pending() {
			subscriptions.push(Subscription::run(resize_tick_stream).map(Message::ResizeTick));
		}

		Subscription::batch(subscriptions)
	}

	pub(crate) fn update(&mut self, message: Message) -> Task<Message> {
		match message {
			Message::LoadPreset(preset) => self.handle_load_preset(preset),
			Message::FontSelected(font) => {
				self.controls.font = font;
				self.rebuild_scene(SceneRefreshReason::ControlsChanged);
			}
			Message::ShapingSelected(shaping) => {
				self.controls.shaping = shaping;
				self.rebuild_scene(SceneRefreshReason::ControlsChanged);
			}
			Message::WrappingSelected(wrapping) => {
				self.controls.wrapping = wrapping;
				self.rebuild_scene(SceneRefreshReason::ControlsChanged);
			}
			Message::RenderModeSelected(render_mode) => {
				self.controls.render_mode = render_mode;
				self.rebuild_scene(SceneRefreshReason::ControlsChanged);
			}
			Message::FontSizeChanged(font_size) => {
				self.controls.font_size = font_size;
				self.controls.line_height = self.controls.line_height.max(self.controls.font_size);
				self.rebuild_scene(SceneRefreshReason::ControlsChanged);
			}
			Message::LineHeightChanged(line_height) => {
				self.controls.line_height = line_height;
				self.rebuild_scene(SceneRefreshReason::ControlsChanged);
			}
			Message::CanvasViewportResized(size) => self.handle_canvas_viewport_resized(size),
			Message::ResizeTick(now) => {
				if self.viewport.flush_resize(now).is_some() {
					self.rebuild_scene(SceneRefreshReason::ResizeReflow);
				}
			}
			Message::ShowBaselinesChanged(show_baselines) => {
				self.controls.show_baselines = show_baselines;
				self.viewport.scene_revision += 1;
			}
			Message::ShowHitboxesChanged(show_hitboxes) => {
				self.controls.show_hitboxes = show_hitboxes;
				self.viewport.scene_revision += 1;
			}
			Message::SelectSidebarTab(tab) => {
				self.sidebar.set_active_tab(tab, self.session.scene());
			}
			Message::PerfTick(_now) => {}
			Message::CanvasHovered(target) => {
				self.sidebar.set_hovered_target(target);
			}
			Message::CanvasScrollChanged(scroll) => {
				self.viewport.canvas_scroll = scroll;
			}
			Message::CanvasPressed {
				target,
				position,
				double_click,
			} => {
				self.sidebar.set_selected_target(target);
				self.dispatch_editor_command(
					EditorCommand::BeginPointerSelection {
						position,
						select_word: double_click,
					},
					EditorDispatchSource::PointerPress,
				);
			}
			Message::CanvasDragged(position) => {
				self.dispatch_editor_command(
					EditorCommand::DragPointerSelection(position),
					EditorDispatchSource::PointerDrag,
				);
			}
			Message::CanvasReleased => {
				self.dispatch_editor_command(EditorCommand::EndPointerSelection, EditorDispatchSource::PointerRelease);
			}
			Message::PaneResized(event) => {
				self.shell.chrome.resize(event.split, event.ratio);
			}
			Message::EditorCommand(command) => {
				self.dispatch_editor_command(command, EditorDispatchSource::Keyboard);
			}
		}

		self.perf.flush_canvas_metrics();
		Task::none()
	}

	fn handle_load_preset(&mut self, preset: SamplePreset) {
		self.controls.preset = preset;

		if matches!(preset, SamplePreset::Custom) {
			return;
		}

		let config = self.scene_config();
		let started = Instant::now();
		self.session.reset_with_preset(preset.text(), config);
		self.viewport.mark_scene_applied(Instant::now());
		self.finish_scene_refresh(SceneRefreshReason::PresetLoaded, started.elapsed());
	}

	fn handle_canvas_viewport_resized(&mut self, size: Size) {
		let now = Instant::now();
		let (width_changed, refresh_ready) = self.viewport.observe_resize(size, now);

		if width_changed {
			self.session.sync_width(self.viewport.layout_width);
		}

		self.viewport.clamp_scroll(self.session.scene());

		if refresh_ready.is_some() {
			self.rebuild_scene(SceneRefreshReason::ResizeReflow);
		}
	}

	fn dispatch_editor_command(&mut self, command: EditorCommand, source: EditorDispatchSource) {
		let command_started = Instant::now();
		let apply_started = Instant::now();
		let update = self.session.apply_editor_command(command);
		self.perf.record_editor_apply(apply_started.elapsed());

		if update.document_changed {
			self.controls.preset = SamplePreset::Custom;
		}

		if update.scene_needs_rebuild {
			self.rebuild_scene(SceneRefreshReason::DocumentEdited);
		}

		if source.reveals_viewport() && update.view_changed {
			self.viewport
				.reveal_target(self.session.view_state().viewport_target, self.session.scene());
		}

		self.perf.record_editor_command(command_started.elapsed());
	}

	fn rebuild_scene(&mut self, reason: SceneRefreshReason) {
		let config = self.scene_config();
		let started = Instant::now();
		self.session.rebuild(config);
		self.viewport.mark_scene_applied(Instant::now());
		self.finish_scene_refresh(reason, started.elapsed());
	}

	fn finish_scene_refresh(&mut self, reason: SceneRefreshReason, duration: Duration) {
		self.sidebar.sync_after_scene_refresh(self.session.scene());
		self.viewport
			.finish_scene_refresh(self.session.scene(), reason.resets_scroll());
		self.perf.record_scene_build(duration);

		if reason.records_resize_reflow() {
			self.perf.record_resize_reflow(duration);
		}
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
