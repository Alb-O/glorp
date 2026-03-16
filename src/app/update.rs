use {
	super::{
		EditorApp,
		session::DocumentUpdate,
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
	fn resets_scroll(self) -> bool {
		matches!(self, Self::PresetLoaded | Self::ControlsChanged)
	}

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
				self.finish_decoration_toggle();
			}
			ControlsMessage::ShowHitboxesChanged(show_hitboxes) => {
				if self.controls.show_hitboxes == show_hitboxes {
					return;
				}
				self.controls.show_hitboxes = show_hitboxes;
				self.finish_decoration_toggle();
			}
		}
	}

	fn handle_sidebar_message(&mut self, message: SidebarMessage) {
		match message {
			SidebarMessage::SelectTab(tab) => {
				if self.sidebar.active_tab == tab {
					return;
				}
				let was_inspect = self.sidebar.active_tab == SidebarTab::Inspect;
				self.sidebar.set_active_tab(tab);
				// The inspect pane is the only consumer of this cache, so unrelated
				// tab switches should not force it cold.
				if was_inspect || tab == SidebarTab::Inspect {
					self.sidebar_cache.invalidate_inspect();
				}
				if matches!(tab, SidebarTab::Inspect | SidebarTab::Perf) {
					self.ensure_scene_for_active_consumers(None);
				}
			}
		}
	}

	fn handle_canvas_message(&mut self, message: CanvasEvent) {
		match message {
			CanvasEvent::Hovered(target) => {
				let target = self.inspect_target(target);
				let previous = self.sidebar.hovered_target;
				self.sidebar.set_hovered_target(target);
				if self.sidebar.hovered_target != previous {
					self.sidebar_cache.invalidate_inspect();
				}
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
				let target = self.inspect_target(target);
				let previous = self.sidebar.selected_target;
				self.sidebar.set_selected_target(target);
				// Selection can be normalized inside the sidebar state, so compare the
				// stored target rather than the incoming payload before invalidating.
				if self.sidebar.selected_target != previous {
					self.sidebar_cache.invalidate_inspect();
				}
				self.dispatch_editor_intent(EditorIntent::Pointer(intent), EditorDispatchSource::PointerPress);
			}
		}
	}

	fn handle_viewport_message(&mut self, message: ViewportMessage) {
		match message {
			ViewportMessage::CanvasResized(size) => self.handle_canvas_viewport_resized(size),
			ViewportMessage::ResizeTick(_now) => {
				if let Some(width) = self.viewport.flush_resize() {
					self.sync_editor_width(width);
				}
			}
		}
	}

	fn handle_load_preset(&mut self, preset: SamplePreset) {
		self.controls.preset = preset;

		if matches!(preset, SamplePreset::Custom) {
			return;
		}

		let started = Instant::now();
		self.session.reset_with_preset(preset.text(), self.scene_config());
		self.viewport.mark_scene_applied();
		self.finish_session_refresh(SceneRefreshReason::PresetLoaded);
		let elapsed = started.elapsed();
		trace!(
			preset = %preset,
			duration_ms = duration_ms(elapsed),
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
		if !self.session.sync_config(self.scene_config()) {
			return;
		}

		self.viewport.mark_scene_applied();
		self.finish_session_refresh(reason);
	}

	fn sync_editor_width(&mut self, width: f32) {
		let started = Instant::now();
		if !self.session.sync_width(width) {
			return;
		}

		let elapsed = started.elapsed();
		self.perf.record_editor_width_sync(elapsed);
		self.viewport.mark_scene_applied();
		self.finish_session_refresh(SceneRefreshReason::ResizeReflow);
	}

	fn dispatch_editor_intent(&mut self, intent: EditorIntent, source: EditorDispatchSource) {
		let _span = trace_span!("editor.intent", intent = ?intent, source = ?source).entered();
		let command_started = Instant::now();
		let apply_started = Instant::now();
		let outcome = self.session.apply_editor_intent(intent);
		let document_changed = outcome.document_changed();
		let view_changed = outcome.view_changed();
		let selection_changed = outcome.selection_changed();
		let mode_changed = outcome.mode_changed();
		let apply_elapsed = apply_started.elapsed();
		self.perf.record_editor_apply(apply_elapsed);
		self.handle_document_update(&outcome, source);
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

	fn handle_document_update(&mut self, outcome: &DocumentUpdate, source: EditorDispatchSource) {
		if outcome.document_changed() {
			self.controls.preset = SamplePreset::Custom;
			self.finish_session_refresh(SceneRefreshReason::DocumentEdited);
			self.sidebar_cache.invalidate_perf();
		}

		if outcome.view_changed() || outcome.selection_changed() || outcome.mode_changed() {
			self.sidebar_cache.invalidate_inspect();
		}

		if outcome.mode_changed() {
			self.sidebar_cache.invalidate_perf();
		}

		if source.reveals_viewport() && outcome.view_changed() {
			self.viewport
				.reveal_target_with_metrics(outcome.viewport_target, self.session.viewport_metrics());
		}
	}

	fn finish_presentation_refresh(&mut self, reason: SceneRefreshReason, duration: Option<Duration>) {
		let Some(layout) = self.session.derived_scene_layout() else {
			return;
		};
		self.sidebar.sync_after_scene_refresh();
		self.viewport.finish_scene_refresh(layout, reason.resets_scroll());
		self.viewport.scene_revision += 1;
		self.sidebar_cache.invalidate_scene_derived();

		if let Some(duration) = duration {
			self.perf.record_scene_build(duration);
			if reason.records_resize_reflow() {
				self.perf.record_resize_reflow(duration);
			}
			log_scene_refresh("scene rebuilt", duration, layout);
		}
	}

	fn finish_decoration_toggle(&mut self) {
		self.sidebar_cache.invalidate_scene_derived();
		if self.derived_scene_consumer_active() {
			if self.ensure_scene_for_active_consumers(None) {
				return;
			}
			self.viewport.scene_revision += 1;
		}
	}

	fn finish_editor_refresh(&mut self, reset_scroll: bool) {
		self.viewport
			.finish_editor_refresh(self.session.viewport_metrics(), reset_scroll);
		self.sidebar_cache.invalidate_scene_derived();
	}

	fn finish_session_refresh(&mut self, reason: SceneRefreshReason) {
		// Prefer the hot path by default. The scene is only rebuilt when a live
		// consumer is active; otherwise editor metrics are enough to keep the UI
		// coherent.
		if !self.ensure_scene_for_active_consumers(Some(reason)) {
			self.finish_editor_refresh(reason.resets_scroll());
		}
	}

	fn ensure_scene_for_active_consumers(&mut self, reason: Option<SceneRefreshReason>) -> bool {
		if !self.derived_scene_consumer_active() {
			return false;
		}

		let Some(duration) = self.session.ensure_derived_scene() else {
			return false;
		};

		if let Some(reason) = reason {
			// Width/config/document-driven scene work is still worth recording as a
			// real scene build when it happens for an active consumer.
			self.finish_presentation_refresh(reason, Some(duration));
		} else {
			// Tab switches and similar "cold start" accesses should refresh the
			// scene cache without back-filling perf metrics that imply an edit or
			// resize caused the work.
			self.finish_presentation_refresh(SceneRefreshReason::DocumentEdited, None);
		}

		true
	}

	fn derived_scene_consumer_active(&self) -> bool {
		matches!(self.sidebar.active_tab, SidebarTab::Inspect | SidebarTab::Perf)
			|| self.controls.show_baselines
			|| self.controls.show_hitboxes
	}

	fn inspect_targeting_active(&self) -> bool {
		self.sidebar.active_tab == SidebarTab::Inspect
	}

	fn inspect_target(&self, target: Option<crate::types::CanvasTarget>) -> Option<crate::types::CanvasTarget> {
		// Keep inspect hover/selection state fully dormant outside the Inspect
		// tab so normal editing never depends on scene hit testing.
		target.filter(|_| self.inspect_targeting_active())
	}
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
