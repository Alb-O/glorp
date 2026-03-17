use {
	super::{
		EditorApp,
		session::{SceneDemand, ScrollIntent, SessionUpdate},
		state::{EditorDispatchSource, RESIZE_REFLOW_INTERVAL},
	},
	crate::{
		editor::{EditorIntent, EditorPointerIntent},
		telemetry::duration_ms,
		types::{
			CanvasEvent, ControlsMessage, Message, PerfMessage, SamplePreset, ShellMessage, SidebarMessage, SidebarTab,
			ViewportMessage,
		},
	},
	iced::{Size, Subscription, Task, time},
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
	fn records_resize_reflow(self) -> bool {
		matches!(self, Self::ResizeReflow)
	}
}

impl EditorApp {
	pub(crate) fn subscription(&self) -> Subscription<Message> {
		let perf = self.sidebar.active_tab == SidebarTab::Perf;
		let resize = self.viewport.resize_coalescer.has_pending();

		match (perf, resize) {
			(false, false) => Subscription::none(),
			(true, false) => time::every(Duration::from_millis(100)).map(|now| Message::Perf(PerfMessage::Tick(now))),
			(false, true) => {
				time::every(RESIZE_REFLOW_INTERVAL).map(|now| Message::Viewport(ViewportMessage::ResizeTick(now)))
			}
			(true, true) => Subscription::batch([
				time::every(Duration::from_millis(100)).map(|now| Message::Perf(PerfMessage::Tick(now))),
				time::every(RESIZE_REFLOW_INTERVAL).map(|now| Message::Viewport(ViewportMessage::ResizeTick(now))),
			]),
		}
	}

	pub(crate) fn update(&mut self, message: Message) -> Task<Message> {
		let _span = trace_span!("editor_app.update", message = message_kind(&message)).entered();

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
				if self.controls.font == font {
					return;
				}
				self.controls.font = font;
				self.refresh_session_config(SceneRefreshReason::ControlsChanged);
			}
			ControlsMessage::ShapingSelected(shaping) => {
				if self.controls.shaping == shaping {
					return;
				}
				self.controls.shaping = shaping;
				self.refresh_session_config(SceneRefreshReason::ControlsChanged);
			}
			ControlsMessage::WrappingSelected(wrapping) => {
				if self.controls.wrapping == wrapping {
					return;
				}
				self.controls.wrapping = wrapping;
				self.refresh_session_config(SceneRefreshReason::ControlsChanged);
			}
			ControlsMessage::FontSizeChanged(font_size) => {
				if (self.controls.font_size - font_size).abs() < f32::EPSILON {
					return;
				}
				self.controls.font_size = font_size;
				self.controls.line_height = self.controls.line_height.max(self.controls.font_size);
				self.refresh_session_config(SceneRefreshReason::ControlsChanged);
			}
			ControlsMessage::LineHeightChanged(line_height) => {
				if (self.controls.line_height - line_height).abs() < f32::EPSILON {
					return;
				}
				self.controls.line_height = line_height;
				self.refresh_session_config(SceneRefreshReason::ControlsChanged);
			}
			ControlsMessage::ShowBaselinesChanged(show_baselines) => {
				if self.controls.show_baselines == show_baselines {
					return;
				}
				self.controls.show_baselines = show_baselines;
				self.refresh_scene_demand();
			}
			ControlsMessage::ShowHitboxesChanged(show_hitboxes) => {
				if self.controls.show_hitboxes == show_hitboxes {
					return;
				}
				self.controls.show_hitboxes = show_hitboxes;
				self.refresh_scene_demand();
			}
		}
	}

	fn handle_sidebar_message(&mut self, message: SidebarMessage) {
		let SidebarMessage::SelectTab(tab) = message;
		if self.sidebar.active_tab == tab {
			return;
		}

		self.sidebar.set_active_tab(tab);
		if matches!(tab, SidebarTab::Inspect | SidebarTab::Perf) {
			self.refresh_scene_demand();
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
			ViewportMessage::ResizeTick(_now) => self
				.viewport
				.flush_resize()
				.map_or((), |width| self.sync_editor_width(width)),
		}
	}

	fn handle_load_preset(&mut self, preset: SamplePreset) {
		self.controls.preset = preset;

		if matches!(preset, SamplePreset::Custom) {
			return;
		}

		let started = Instant::now();
		let update = self
			.session
			.reset_with_preset(preset.text(), self.scene_config(), self.scene_demand());
		self.viewport.mark_scene_applied();
		self.apply_session_update(update, None, Some(SceneRefreshReason::PresetLoaded));
		trace!(
			preset = %preset,
			duration_ms = duration_ms(started.elapsed()),
			text_bytes = self.session.text().len(),
			"preset loaded"
		);
	}

	fn handle_canvas_viewport_resized(&mut self, size: Size) {
		let width_changed = self.viewport.observe_resize(size);

		self.viewport.clamp_scroll_to_metrics(self.session.viewport_metrics());

		if width_changed {
			trace!(
				canvas_width = size.width,
				canvas_height = size.height,
				layout_width = self.viewport.layout_width,
				width_changed,
				"canvas viewport resized"
			);
		}
	}

	fn refresh_session_config(&mut self, reason: SceneRefreshReason) {
		let update = self.session.sync_config(self.scene_config(), self.scene_demand());
		if !update.changed() && update.scene_build.is_none() {
			return;
		}

		self.viewport.mark_scene_applied();
		self.apply_session_update(update, None, Some(reason));
	}

	fn sync_editor_width(&mut self, width: f32) {
		let update = self.session.sync_width(width, self.scene_demand());
		if update.width_sync.is_none() && update.scene_build.is_none() {
			return;
		}

		self.viewport.mark_scene_applied();
		self.apply_session_update(update, None, Some(SceneRefreshReason::ResizeReflow));
	}

	fn dispatch_editor_intent(&mut self, intent: EditorIntent, source: EditorDispatchSource) {
		let _span = trace_span!("editor.intent", intent = ?intent, source = ?source).entered();
		let command_started = Instant::now();
		let apply_started = Instant::now();
		let update = self.session.apply_editor_intent(intent, self.scene_demand());
		let document_changed = update.document_changed();
		let view_changed = update.view_changed();
		let selection_changed = update.selection_changed();
		let mode_changed = update.mode_changed();
		let apply_elapsed = apply_started.elapsed();
		self.perf.record_editor_apply(apply_elapsed);
		self.apply_session_update(
			update,
			Some(source),
			document_changed.then_some(SceneRefreshReason::DocumentEdited),
		);
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
				text_bytes = self.session.text().len(),
				"editor command applied"
			);
		}
	}

	fn refresh_scene_demand(&mut self) {
		let update = self.session.ensure_scene(self.scene_demand());
		if update.scene_build.is_some() {
			self.apply_session_update(update, None, None);
		}
	}

	fn scene_demand(&self) -> SceneDemand {
		if self.derived_scene_consumer_active() {
			SceneDemand::DerivedRequired
		} else {
			SceneDemand::HotOnly
		}
	}

	fn apply_session_update(
		&mut self, update: SessionUpdate, source: Option<EditorDispatchSource>, reason: Option<SceneRefreshReason>,
	) {
		if let Some(duration) = update.width_sync {
			self.perf.record_editor_width_sync(duration);
		}

		if update.document_changed() {
			self.controls.preset = SamplePreset::Custom;
		}

		let reset_scroll = matches!(update.scroll_intent, ScrollIntent::ResetScroll);
		if let Some(scene) = self
			.session
			.snapshot()
			.scene
			.as_ref()
			.filter(|_| update.scene_build.is_some())
		{
			self.sidebar.sync_after_scene_refresh();
			self.viewport.finish_scene_refresh(scene.layout.as_ref(), reset_scroll);

			if let Some(reason) = reason {
				self.perf.record_scene_build(duration_or_zero(update.scene_build));
				if reason.records_resize_reflow() {
					self.perf.record_resize_reflow(duration_or_zero(update.scene_build));
				}
				log_scene_refresh(
					"scene rebuilt",
					duration_or_zero(update.scene_build),
					scene.layout.as_ref(),
				);
			}
		} else {
			self.viewport
				.finish_editor_refresh(self.session.viewport_metrics(), reset_scroll);
		}

		if source.is_some_and(EditorDispatchSource::reveals_viewport)
			&& update.view_changed()
			&& matches!(update.scroll_intent, ScrollIntent::KeepClamped)
		{
			self.viewport
				.reveal_target_with_metrics(update.viewport_target, self.session.viewport_metrics());
		}
	}

	fn derived_scene_consumer_active(&self) -> bool {
		matches!(self.sidebar.active_tab, SidebarTab::Inspect | SidebarTab::Perf)
			|| self.controls.show_baselines
			|| self.controls.show_hitboxes
	}
}

fn duration_or_zero(duration: Option<Duration>) -> Duration {
	duration.unwrap_or_default()
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

fn log_scene_refresh(label: &str, elapsed: Duration, layout: &crate::scene::DocumentLayout) {
	let elapsed_ms = duration_ms(elapsed);
	if elapsed_ms >= 16.7 {
		warn!(
			duration_ms = elapsed_ms,
			scene_width = layout.measured_width,
			scene_height = layout.measured_height,
			"{label} over frame budget"
		);
	} else if elapsed_ms >= 8.0 {
		debug!(
			duration_ms = elapsed_ms,
			scene_width = layout.measured_width,
			scene_height = layout.measured_height,
			"{label} over warning threshold"
		);
	} else {
		trace!(
			duration_ms = elapsed_ms,
			scene_width = layout.measured_width,
			scene_height = layout.measured_height,
			"{label}"
		);
	}
}
