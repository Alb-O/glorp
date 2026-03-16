use {
	super::{
		Playground,
		state::{EditorDispatchSource, RESIZE_REFLOW_INTERVAL},
	},
	crate::{
		editor::{EditorIntent, EditorOutcome, EditorPointerIntent},
		telemetry::duration_ms,
		types::{
			CanvasEvent, ControlsMessage, Message, PerfMessage, SamplePreset, ShellMessage, SidebarMessage, SidebarTab,
			ViewportMessage,
		},
	},
	iced::{Size, Subscription, Task, futures, stream},
	std::time::{Duration, Instant},
	tracing::{debug, trace, trace_span, warn},
};

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
		let _span = trace_span!("playground.update", message = message_kind(&message)).entered();

		match message {
			Message::Controls(message) => self.handle_controls_message(message),
			Message::Sidebar(message) => self.handle_sidebar_message(message),
			Message::Canvas(message) => self.handle_canvas_message(message),
			Message::Editor(intent) => {
				let source = editor_dispatch_source(&intent);
				self.dispatch_editor_intent(intent, source);
			}
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
				if show_baselines && self.scene_dirty {
					self.rebuild_scene(SceneRefreshReason::ControlsChanged);
				} else {
					self.viewport.scene_revision += 1;
				}
			}
			ControlsMessage::ShowHitboxesChanged(show_hitboxes) => {
				self.controls.show_hitboxes = show_hitboxes;
				if show_hitboxes && self.scene_dirty {
					self.rebuild_scene(SceneRefreshReason::ControlsChanged);
				} else {
					self.viewport.scene_revision += 1;
				}
			}
		}
	}

	fn handle_sidebar_message(&mut self, message: SidebarMessage) {
		match message {
			SidebarMessage::SelectTab(tab) => {
				self.sidebar.set_active_tab(tab);
				self.ensure_scene_current(SceneRefreshReason::DocumentEdited);
			}
		}
	}

	fn handle_canvas_message(&mut self, message: CanvasEvent) {
		match message {
			CanvasEvent::Hovered(target) => {
				self.sidebar.set_hovered_target(target);
			}
			CanvasEvent::FocusChanged(focused) => {
				self.viewport.canvas_focused = focused;
			}
			CanvasEvent::ScrollChanged(scroll) => {
				self.viewport.canvas_focused = true;
				self.viewport.canvas_scroll = scroll;
			}
			CanvasEvent::PointerSelectionStarted { target, intent } => {
				self.viewport.canvas_focused = true;
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
					self.rebuild_scene_or_defer(SceneRefreshReason::ResizeReflow);
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
		self.scene_dirty = false;
		let elapsed = started.elapsed();
		self.finish_scene_refresh(SceneRefreshReason::PresetLoaded, elapsed);
		trace!(
			preset = %preset,
			duration_ms = duration_ms(elapsed),
			text_bytes = self.session.text().len(),
			"preset loaded"
		);
	}

	fn handle_canvas_viewport_resized(&mut self, size: Size) {
		let now = Instant::now();
		let (width_changed, refresh_ready) = self.viewport.observe_resize(size, now);

		if width_changed {
			self.session.sync_width(self.viewport.layout_width);
		}

		self.viewport.clamp_scroll_to_metrics(self.session.viewport_metrics());

		if width_changed || refresh_ready.is_some() {
			trace!(
				canvas_width = size.width,
				canvas_height = size.height,
				layout_width = self.viewport.layout_width,
				width_changed,
				refresh_ready = refresh_ready.is_some(),
				"canvas viewport resized"
			);
		}

		if refresh_ready.is_some() {
			self.rebuild_scene_or_defer(SceneRefreshReason::ResizeReflow);
		}
	}

	fn dispatch_editor_intent(&mut self, intent: EditorIntent, source: EditorDispatchSource) {
		let _span = trace_span!("editor.intent", intent = ?intent, source = ?source).entered();
		let command_started = Instant::now();
		let apply_started = Instant::now();
		let outcome = self.session.apply_editor_intent(intent);
		let document_changed = outcome.document_changed();
		let view_changed = outcome.view_changed;
		let selection_changed = outcome.selection_changed;
		let mode_changed = outcome.mode_changed;
		let requires_scene_rebuild = outcome.requires_scene_rebuild();
		let apply_elapsed = apply_started.elapsed();
		self.perf.record_editor_apply(apply_elapsed);
		self.handle_editor_outcome(&outcome, source);
		let command_elapsed = command_started.elapsed();
		self.perf.record_editor_command(command_elapsed);

		let apply_ms = duration_ms(apply_elapsed);
		let command_ms = duration_ms(command_elapsed);

		if command_ms >= 16.7 {
			warn!(
				apply_ms,
				command_ms,
				document_changed,
				view_changed,
				selection_changed,
				mode_changed,
				requires_scene_rebuild,
				text_bytes = self.session.text().len(),
				"editor command over frame budget"
			);
		} else if command_ms >= 8.0 {
			debug!(
				apply_ms,
				command_ms,
				document_changed,
				view_changed,
				selection_changed,
				mode_changed,
				requires_scene_rebuild,
				text_bytes = self.session.text().len(),
				"editor command over warning threshold"
			);
		} else {
			trace!(
				apply_ms,
				command_ms,
				document_changed,
				view_changed,
				selection_changed,
				mode_changed,
				requires_scene_rebuild,
				text_bytes = self.session.text().len(),
				"editor command applied"
			);
		}
	}

	fn handle_editor_outcome(&mut self, outcome: &EditorOutcome, source: EditorDispatchSource) {
		if outcome.document_changed() {
			self.controls.preset = SamplePreset::Custom;
		}

		if outcome.requires_scene_rebuild() {
			self.rebuild_scene_or_defer(SceneRefreshReason::DocumentEdited);
		}

		if source.reveals_viewport() && outcome.view_changed {
			self.viewport
				.reveal_target_with_metrics(outcome.viewport_target, self.session.viewport_metrics());
		}
	}

	fn rebuild_scene(&mut self, reason: SceneRefreshReason) {
		let _span = trace_span!("scene.rebuild", reason = ?reason).entered();
		let config = self.scene_config();
		let started = Instant::now();
		self.session.rebuild(config);
		self.viewport.mark_scene_applied(Instant::now());
		self.scene_dirty = false;
		self.deferred_resize_reflow = false;
		let elapsed = started.elapsed();
		self.finish_scene_refresh(reason, elapsed);

		let elapsed_ms = duration_ms(elapsed);
		if elapsed_ms >= 16.7 {
			warn!(
				duration_ms = elapsed_ms,
				scene_width = self.session.scene().measured_width,
				scene_height = self.session.scene().measured_height,
				"scene rebuild over frame budget"
			);
		} else if elapsed_ms >= 8.0 {
			debug!(
				duration_ms = elapsed_ms,
				scene_width = self.session.scene().measured_width,
				scene_height = self.session.scene().measured_height,
				"scene rebuild over warning threshold"
			);
		} else {
			trace!(
				duration_ms = elapsed_ms,
				scene_width = self.session.scene().measured_width,
				scene_height = self.session.scene().measured_height,
				"scene rebuilt"
			);
		}
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

	fn ensure_scene_current(&mut self, reason: SceneRefreshReason) {
		if self.scene_dirty && self.requires_immediate_scene_refresh() {
			self.rebuild_scene(self.pending_scene_refresh_reason(reason));
		}
	}

	fn rebuild_scene_or_defer(&mut self, reason: SceneRefreshReason) {
		if self.requires_immediate_scene_refresh() {
			self.rebuild_scene(self.pending_scene_refresh_reason(reason));
			return;
		}

		self.scene_dirty = true;
		self.deferred_resize_reflow |= matches!(reason, SceneRefreshReason::ResizeReflow);
	}

	fn pending_scene_refresh_reason(&self, fallback: SceneRefreshReason) -> SceneRefreshReason {
		if self.deferred_resize_reflow {
			SceneRefreshReason::ResizeReflow
		} else {
			fallback
		}
	}

	fn requires_immediate_scene_refresh(&self) -> bool {
		self.sidebar.active_tab != SidebarTab::Controls
			|| self.controls.show_baselines
			|| self.controls.show_hitboxes
			|| self.controls.render_mode.draw_outlines()
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
		EditorIntent::Pointer(EditorPointerIntent::Begin { .. }) => EditorDispatchSource::PointerPress,
		EditorIntent::Pointer(EditorPointerIntent::Drag(_)) => EditorDispatchSource::PointerDrag,
		EditorIntent::Pointer(EditorPointerIntent::End) => EditorDispatchSource::PointerRelease,
		_ => EditorDispatchSource::Keyboard,
	}
}

fn message_kind(message: &Message) -> &'static str {
	match message {
		Message::Controls(_) => "controls",
		Message::Sidebar(_) => "sidebar",
		Message::Canvas(_) => "canvas",
		Message::Editor(_) => "editor",
		Message::Perf(_) => "perf",
		Message::Viewport(_) => "viewport",
		Message::Shell(_) => "shell",
	}
}
