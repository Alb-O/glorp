use {
	super::{
		AppModel,
		session::SessionDelta,
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

#[derive(Debug, Clone)]
pub(super) enum AppAction {
	Controls(ControlsMessage),
	ReplaceDocument {
		text: String,
		preset: SamplePreset,
	},
	Sidebar(SidebarMessage),
	Canvas(CanvasEvent),
	Editor {
		intent: EditorIntent,
		source: EditorDispatchSource,
	},
	PerfTick,
	Viewport(ViewportMessage),
	Shell(ShellMessage),
}

impl From<Message> for AppAction {
	fn from(message: Message) -> Self {
		match message {
			Message::Controls(message) => Self::Controls(message),
			Message::Sidebar(message) => Self::Sidebar(message),
			Message::Canvas(message) => Self::Canvas(message),
			Message::Editor(intent) => Self::Editor {
				source: editor_dispatch_source(&intent),
				intent,
			},
			Message::Perf(PerfMessage::Tick(_now)) => Self::PerfTick,
			Message::Viewport(message) => Self::Viewport(message),
			Message::Shell(message) => Self::Shell(message),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScrollBehavior {
	KeepClamped,
	ResetScroll,
}

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

#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct AppTransition {
	pub(super) session: SessionDelta,
	pub(super) scene_build: Option<Duration>,
	scroll_behavior: ScrollBehavior,
	reveal_viewport: bool,
	scene_refresh_reason: Option<SceneRefreshReason>,
}

impl Default for ScrollBehavior {
	fn default() -> Self {
		Self::KeepClamped
	}
}

impl AppTransition {
	fn changed(&self) -> bool {
		self.session.changed() || self.session.width_sync.is_some() || self.scene_build.is_some()
	}
}

impl AppModel {
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
		let _ = self.dispatch(message.into());
		self.perf.flush_canvas_metrics();
		Task::none()
	}

	pub(super) fn dispatch(&mut self, action: AppAction) -> AppTransition {
		match action {
			AppAction::Controls(message) => {
				let transition = self.transition_for_controls(message);
				self.dispatch_transition(transition)
			}
			AppAction::ReplaceDocument { text, preset } => {
				let transition = self.transition_for_document_replacement(text, preset);
				self.dispatch_transition(transition)
			}
			AppAction::Sidebar(message) => {
				let transition = self.transition_for_sidebar(message);
				self.dispatch_transition(transition)
			}
			AppAction::Canvas(message) => self.dispatch_canvas(message),
			AppAction::Editor { intent, source } => self.dispatch_editor(intent, source),
			AppAction::PerfTick => AppTransition::default(),
			AppAction::Viewport(message) => {
				let transition = self.transition_for_viewport(message);
				self.dispatch_transition(transition)
			}
			AppAction::Shell(ShellMessage::PaneResized(event)) => {
				self.shell.chrome.resize(event.split, event.ratio);
				AppTransition::default()
			}
		}
	}

	pub(super) fn ensure_scene_ready(&mut self) -> AppTransition {
		let transition = self.transition_for_scene_refresh();
		self.dispatch_transition(transition)
	}

	fn dispatch_transition(&mut self, transition: AppTransition) -> AppTransition {
		self.apply_transition(&transition);
		transition
	}

	fn dispatch_canvas(&mut self, message: CanvasEvent) -> AppTransition {
		match message {
			CanvasEvent::Hovered(target) => {
				self.sidebar.set_hovered_target(target);
				AppTransition::default()
			}
			CanvasEvent::FocusChanged(focused) => {
				self.viewport.canvas_focused = focused;
				AppTransition::default()
			}
			CanvasEvent::ScrollChanged(scroll) => {
				self.viewport.canvas_focused = true;
				self.viewport.canvas_scroll = scroll;
				AppTransition::default()
			}
			CanvasEvent::PointerSelectionStarted { target, intent } => {
				self.viewport.canvas_focused = true;
				self.sidebar.set_selected_target(target);
				self.dispatch_editor(intent.into(), EditorDispatchSource::PointerPress)
			}
		}
	}

	fn dispatch_editor(&mut self, intent: EditorIntent, source: EditorDispatchSource) -> AppTransition {
		let _span = trace_span!("editor.intent", intent = ?intent, source = ?source).entered();
		let command_started = Instant::now();
		let apply_started = Instant::now();
		let transition = self.transition_for_editor_intent(intent, source);
		let apply_elapsed = apply_started.elapsed();
		self.perf.record_editor_apply(apply_elapsed);
		self.apply_transition(&transition);
		let command_elapsed = command_started.elapsed();
		self.perf.record_editor_command(command_elapsed);

		let apply_ms = duration_ms(apply_elapsed);
		let command_ms = duration_ms(command_elapsed);
		let document_changed = transition.session.document_changed();
		let view_changed = transition.session.view_changed();
		let selection_changed = transition.session.selection_changed();
		let mode_changed = transition.session.mode_changed();

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

		transition
	}

	fn transition_for_controls(&mut self, message: ControlsMessage) -> AppTransition {
		match message {
			ControlsMessage::LoadPreset(preset) => self.transition_for_preset(preset),
			ControlsMessage::FontSelected(font) => {
				if self.controls.font == font {
					return AppTransition::default();
				}
				self.controls.font = font;
				let session = self.session.sync_config(self.scene_config());
				self.transition_for_session_delta(
					session,
					ScrollBehavior::ResetScroll,
					Some(SceneRefreshReason::ControlsChanged),
					false,
				)
			}
			ControlsMessage::ShapingSelected(shaping) => {
				if self.controls.shaping == shaping {
					return AppTransition::default();
				}
				self.controls.shaping = shaping;
				let session = self.session.sync_config(self.scene_config());
				self.transition_for_session_delta(
					session,
					ScrollBehavior::ResetScroll,
					Some(SceneRefreshReason::ControlsChanged),
					false,
				)
			}
			ControlsMessage::WrappingSelected(wrapping) => {
				if self.controls.wrapping == wrapping {
					return AppTransition::default();
				}
				self.controls.wrapping = wrapping;
				let session = self.session.sync_config(self.scene_config());
				self.transition_for_session_delta(
					session,
					ScrollBehavior::ResetScroll,
					Some(SceneRefreshReason::ControlsChanged),
					false,
				)
			}
			ControlsMessage::FontSizeChanged(font_size) => {
				if (self.controls.font_size - font_size).abs() < f32::EPSILON {
					return AppTransition::default();
				}
				self.controls.font_size = font_size;
				self.controls.line_height = self.controls.line_height.max(self.controls.font_size);
				let session = self.session.sync_config(self.scene_config());
				self.transition_for_session_delta(
					session,
					ScrollBehavior::ResetScroll,
					Some(SceneRefreshReason::ControlsChanged),
					false,
				)
			}
			ControlsMessage::LineHeightChanged(line_height) => {
				if (self.controls.line_height - line_height).abs() < f32::EPSILON {
					return AppTransition::default();
				}
				self.controls.line_height = line_height;
				let session = self.session.sync_config(self.scene_config());
				self.transition_for_session_delta(
					session,
					ScrollBehavior::ResetScroll,
					Some(SceneRefreshReason::ControlsChanged),
					false,
				)
			}
			ControlsMessage::ShowBaselinesChanged(show_baselines) => {
				if self.controls.show_baselines == show_baselines {
					return AppTransition::default();
				}
				self.controls.show_baselines = show_baselines;
				self.transition_for_scene_refresh()
			}
			ControlsMessage::ShowHitboxesChanged(show_hitboxes) => {
				if self.controls.show_hitboxes == show_hitboxes {
					return AppTransition::default();
				}
				self.controls.show_hitboxes = show_hitboxes;
				self.transition_for_scene_refresh()
			}
		}
	}

	fn transition_for_sidebar(&mut self, message: SidebarMessage) -> AppTransition {
		let SidebarMessage::SelectTab(tab) = message;
		if self.sidebar.active_tab == tab {
			return AppTransition::default();
		}

		self.sidebar.set_active_tab(tab);
		if matches!(tab, SidebarTab::Inspect | SidebarTab::Perf) {
			self.transition_for_scene_refresh()
		} else {
			AppTransition::default()
		}
	}

	fn transition_for_viewport(&mut self, message: ViewportMessage) -> AppTransition {
		match message {
			ViewportMessage::CanvasResized(size) => {
				self.handle_canvas_viewport_resized(size);
				AppTransition::default()
			}
			ViewportMessage::ResizeTick(_now) => {
				self.viewport
					.flush_resize()
					.map_or_else(AppTransition::default, |width| {
						let session = self.session.sync_width(width);
						self.transition_for_session_delta(
							session,
							ScrollBehavior::KeepClamped,
							Some(SceneRefreshReason::ResizeReflow),
							false,
						)
					})
			}
		}
	}

	fn transition_for_preset(&mut self, preset: SamplePreset) -> AppTransition {
		self.controls.preset = preset;
		if matches!(preset, SamplePreset::Custom) {
			return AppTransition::default();
		}

		let started = Instant::now();
		let session = self.session.reset_with_preset(preset.text(), self.scene_config());
		let transition = self.transition_for_session_delta(
			session,
			ScrollBehavior::ResetScroll,
			Some(SceneRefreshReason::PresetLoaded),
			false,
		);
		trace!(
			preset = %preset,
			duration_ms = duration_ms(started.elapsed()),
			text_bytes = self.session.text().len(),
			"preset loaded"
		);
		transition
	}

	fn transition_for_document_replacement(&mut self, text: String, preset: SamplePreset) -> AppTransition {
		self.controls.preset = preset;
		let session = self.session.reset_with_preset(&text, self.scene_config());
		self.transition_for_session_delta(
			session,
			ScrollBehavior::ResetScroll,
			Some(SceneRefreshReason::PresetLoaded),
			false,
		)
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

	fn transition_for_editor_intent(&mut self, intent: EditorIntent, source: EditorDispatchSource) -> AppTransition {
		let session = self.session.apply_editor_intent(intent);
		let reason = session.document_changed().then_some(SceneRefreshReason::DocumentEdited);
		let reveal_viewport = source.reveals_viewport() && session.view_changed();

		self.transition_for_session_delta(session, ScrollBehavior::KeepClamped, reason, reveal_viewport)
	}

	fn transition_for_scene_refresh(&mut self) -> AppTransition {
		AppTransition {
			scene_build: self
				.derived_scene_consumer_active()
				.then(|| self.session.ensure_scene())
				.flatten(),
			..AppTransition::default()
		}
	}

	fn transition_for_session_delta(
		&mut self, session: SessionDelta, scroll_behavior: ScrollBehavior, reason: Option<SceneRefreshReason>,
		reveal_viewport: bool,
	) -> AppTransition {
		let scene_build = self
			.derived_scene_consumer_active()
			.then(|| self.session.ensure_scene())
			.flatten();

		AppTransition {
			session,
			scene_build,
			scroll_behavior,
			reveal_viewport,
			scene_refresh_reason: reason,
		}
	}

	fn apply_transition(&mut self, transition: &AppTransition) {
		if !transition.changed()
			&& !transition.reveal_viewport
			&& transition.scroll_behavior == ScrollBehavior::KeepClamped
		{
			return;
		}

		if let Some(duration) = transition.session.width_sync {
			self.perf.record_editor_width_sync(duration);
		}

		if transition.session.document_changed() {
			self.controls.preset = SamplePreset::Custom;
		}

		let reset_scroll = matches!(transition.scroll_behavior, ScrollBehavior::ResetScroll);
		if let Some(scene) = self
			.session
			.snapshot()
			.scene
			.as_ref()
			.filter(|_| transition.scene_build.is_some())
		{
			self.sidebar.sync_after_scene_refresh();
			self.viewport.finish_scene_refresh(scene.layout.as_ref(), reset_scroll);

			if let Some(reason) = transition.scene_refresh_reason {
				self.perf.record_scene_build(duration_or_zero(transition.scene_build));
				if reason.records_resize_reflow() {
					self.perf.record_resize_reflow(duration_or_zero(transition.scene_build));
				}
				log_scene_refresh(
					"scene rebuilt",
					duration_or_zero(transition.scene_build),
					scene.layout.as_ref(),
				);
			}
		} else if transition.session.changed() || reset_scroll {
			self.viewport
				.finish_editor_refresh(self.session.viewport_metrics(), reset_scroll);
		}

		if transition.reveal_viewport && matches!(transition.scroll_behavior, ScrollBehavior::KeepClamped) {
			self.viewport
				.reveal_target_with_metrics(transition.session.viewport_target, self.session.viewport_metrics());
		}
	}

	fn derived_scene_consumer_active(&self) -> bool {
		matches!(self.sidebar.active_tab, SidebarTab::Inspect | SidebarTab::Perf)
			|| self.controls.show_baselines
			|| self.controls.show_hitboxes
	}
}

impl From<EditorPointerIntent> for EditorIntent {
	fn from(intent: EditorPointerIntent) -> Self {
		Self::Pointer(intent)
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
