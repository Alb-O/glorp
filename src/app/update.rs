use iced::{Size, Subscription, Task, futures, stream};

use std::time::{Duration, Instant};

use crate::editor::{EditorIntent, EditorOutcome, EditorPointerIntent};
use crate::types::{
	CanvasEvent, ControlsMessage, Message, PerfMessage, SamplePreset, ShellMessage, SidebarMessage, SidebarTab,
	ViewportMessage,
};

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
			subscriptions.push(Subscription::run(perf_tick_stream).map(|now| Message::Perf(PerfMessage::Tick(now))));
		}

		if self.viewport.resize_coalescer.has_pending() {
			subscriptions.push(
				Subscription::run(resize_tick_stream).map(|now| Message::Viewport(ViewportMessage::ResizeTick(now))),
			);
		}

		Subscription::batch(subscriptions)
	}

	pub(crate) fn update(&mut self, message: Message) -> Task<Message> {
		match message {
			Message::Controls(message) => self.handle_controls_message(message),
			Message::Sidebar(message) => self.handle_sidebar_message(message),
			Message::Canvas(message) => self.handle_canvas_message(message),
			Message::Editor(intent) => self.dispatch_editor_intent(intent.clone(), editor_dispatch_source(&intent)),
			Message::Perf(PerfMessage::Tick(_now)) => {}
			Message::Viewport(message) => self.handle_viewport_message(message),
			Message::Shell(ShellMessage::PaneResized(event)) => {
				self.shell.chrome.resize(event.split, event.ratio);
			}
		}

		self.perf.flush_canvas_metrics();
		Task::none()
	}

	fn handle_controls_message(&mut self, message: ControlsMessage) {
		match message {
			ControlsMessage::LoadPreset(preset) => self.handle_load_preset(preset),
			ControlsMessage::FontSelected(font) => {
				self.controls.font = font;
				self.rebuild_scene(SceneRefreshReason::ControlsChanged);
			}
			ControlsMessage::ShapingSelected(shaping) => {
				self.controls.shaping = shaping;
				self.rebuild_scene(SceneRefreshReason::ControlsChanged);
			}
			ControlsMessage::WrappingSelected(wrapping) => {
				self.controls.wrapping = wrapping;
				self.rebuild_scene(SceneRefreshReason::ControlsChanged);
			}
			ControlsMessage::RenderModeSelected(render_mode) => {
				self.controls.render_mode = render_mode;
				self.rebuild_scene(SceneRefreshReason::ControlsChanged);
			}
			ControlsMessage::FontSizeChanged(font_size) => {
				self.controls.font_size = font_size;
				self.controls.line_height = self.controls.line_height.max(self.controls.font_size);
				self.rebuild_scene(SceneRefreshReason::ControlsChanged);
			}
			ControlsMessage::LineHeightChanged(line_height) => {
				self.controls.line_height = line_height;
				self.rebuild_scene(SceneRefreshReason::ControlsChanged);
			}
			ControlsMessage::ShowBaselinesChanged(show_baselines) => {
				self.controls.show_baselines = show_baselines;
				self.viewport.scene_revision += 1;
			}
			ControlsMessage::ShowHitboxesChanged(show_hitboxes) => {
				self.controls.show_hitboxes = show_hitboxes;
				self.viewport.scene_revision += 1;
			}
		}
	}

	fn handle_sidebar_message(&mut self, message: SidebarMessage) {
		match message {
			SidebarMessage::SelectTab(tab) => self.sidebar.set_active_tab(tab),
		}
	}

	fn handle_canvas_message(&mut self, message: CanvasEvent) {
		match message {
			CanvasEvent::Hovered(target) => {
				self.sidebar.set_hovered_target(target);
			}
			CanvasEvent::ScrollChanged(scroll) => {
				self.viewport.canvas_scroll = scroll;
			}
			CanvasEvent::PointerSelectionStarted { target, intent } => {
				self.sidebar.set_selected_target(target);
				self.dispatch_editor_intent(EditorIntent::Pointer(intent), EditorDispatchSource::PointerPress);
			}
		}
	}

	fn handle_viewport_message(&mut self, message: ViewportMessage) {
		match message {
			ViewportMessage::CanvasResized(size) => self.handle_canvas_viewport_resized(size),
			ViewportMessage::ResizeTick(now) => {
				if self.viewport.flush_resize(now).is_some() {
					self.rebuild_scene(SceneRefreshReason::ResizeReflow);
				}
			}
		}
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

	fn dispatch_editor_intent(&mut self, intent: EditorIntent, source: EditorDispatchSource) {
		let command_started = Instant::now();
		let apply_started = Instant::now();
		let outcome = self.session.apply_editor_intent(intent);
		self.perf.record_editor_apply(apply_started.elapsed());
		self.handle_editor_outcome(outcome, source);
		self.perf.record_editor_command(command_started.elapsed());
	}

	fn handle_editor_outcome(&mut self, outcome: EditorOutcome, source: EditorDispatchSource) {
		if outcome.document_changed {
			self.controls.preset = SamplePreset::Custom;
		}

		if outcome.requires_scene_rebuild {
			self.rebuild_scene(SceneRefreshReason::DocumentEdited);
		}

		if source.reveals_viewport() && outcome.view_changed {
			self.viewport
				.reveal_target(outcome.viewport_target, self.session.scene());
		}
	}

	fn rebuild_scene(&mut self, reason: SceneRefreshReason) {
		let config = self.scene_config();
		let started = Instant::now();
		self.session.rebuild(config);
		self.viewport.mark_scene_applied(Instant::now());
		self.finish_scene_refresh(reason, started.elapsed());
	}

	fn finish_scene_refresh(&mut self, reason: SceneRefreshReason, duration: Duration) {
		self.sidebar.sync_after_scene_refresh();
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

fn editor_dispatch_source(intent: &EditorIntent) -> EditorDispatchSource {
	match intent {
		EditorIntent::Pointer(EditorPointerIntent::BeginSelection { .. }) => EditorDispatchSource::PointerPress,
		EditorIntent::Pointer(EditorPointerIntent::DragSelection(_)) => EditorDispatchSource::PointerDrag,
		EditorIntent::Pointer(EditorPointerIntent::EndSelection) => EditorDispatchSource::PointerRelease,
		_ => EditorDispatchSource::Keyboard,
	}
}
