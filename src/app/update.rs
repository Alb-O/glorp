use {
	super::{
		AppModel,
		session::{SessionCommand, SessionTransition},
		state::{EditorDispatchSource, RESIZE_REFLOW_INTERVAL},
	},
	crate::{
		editor::{EditorIntent, EditorPointerIntent},
		telemetry::duration_ms,
		types::{
			CanvasEvent, CanvasTarget, ControlsMessage, Message, PerfMessage, SamplePreset, ShellMessage,
			SidebarMessage, SidebarTab, ViewportMessage,
		},
	},
	iced::{Size, Subscription, Task, Vector, time, widget::pane_grid},
	std::time::{Duration, Instant},
	tracing::{debug, trace, trace_span, warn},
};

#[derive(Debug, Clone)]
pub(super) enum AppCommand {
	Control(ControlsMessage),
	ReplaceDocument(String),
	SelectSidebarTab(SidebarTab),
	HoverCanvas(Option<CanvasTarget>),
	SetCanvasFocus(bool),
	SetCanvasScroll(Vector),
	BeginPointerSelection {
		target: Option<CanvasTarget>,
		intent: EditorPointerIntent,
	},
	Editor {
		intent: EditorIntent,
		source: EditorDispatchSource,
	},
	PerfTick,
	ObserveCanvasResize(Size),
	FlushResizeReflow,
	ResizePane(pane_grid::ResizeEvent),
}

impl AppCommand {
	pub(super) fn editor(intent: EditorIntent) -> Self {
		Self::Editor {
			source: editor_dispatch_source(&intent),
			intent,
		}
	}
}

impl From<Message> for AppCommand {
	fn from(message: Message) -> Self {
		match message {
			Message::Controls(message) => Self::Control(message),
			Message::Sidebar(SidebarMessage::SelectTab(tab)) => Self::SelectSidebarTab(tab),
			Message::Canvas(CanvasEvent::Hovered(target)) => Self::HoverCanvas(target),
			Message::Canvas(CanvasEvent::FocusChanged(focused)) => Self::SetCanvasFocus(focused),
			Message::Canvas(CanvasEvent::ScrollChanged(scroll)) => Self::SetCanvasScroll(scroll),
			Message::Canvas(CanvasEvent::PointerSelectionStarted { target, intent }) => {
				Self::BeginPointerSelection { target, intent }
			}
			Message::Editor(intent) => Self::editor(intent),
			Message::Perf(PerfMessage::Tick(_now)) => Self::PerfTick,
			Message::Viewport(ViewportMessage::CanvasResized(size)) => Self::ObserveCanvasResize(size),
			Message::Viewport(ViewportMessage::ResizeTick(_now)) => Self::FlushResizeReflow,
			Message::Shell(ShellMessage::PaneResized(event)) => Self::ResizePane(event),
		}
	}
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum ScrollBehavior {
	#[default]
	KeepClamped,
	ResetScroll,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct ApplyPolicy {
	scroll_behavior: ScrollBehavior,
	reveal_viewport: bool,
	scene_refresh_reason: Option<SceneRefreshReason>,
}

impl ApplyPolicy {
	fn keep() -> Self {
		Self::default()
	}

	fn reset_scroll(reason: SceneRefreshReason) -> Self {
		Self {
			scroll_behavior: ScrollBehavior::ResetScroll,
			scene_refresh_reason: Some(reason),
			..Self::default()
		}
	}

	fn scene_refresh(reason: SceneRefreshReason) -> Self {
		Self {
			scene_refresh_reason: Some(reason),
			..Self::default()
		}
	}

	fn reveal(reveal_viewport: bool, scene_refresh_reason: Option<SceneRefreshReason>) -> Self {
		Self {
			reveal_viewport,
			scene_refresh_reason,
			..Self::default()
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SceneRefreshReason {
	PresetLoaded,
	ControlsChanged,
	TextEdited,
	ResizeReflow,
}

impl SceneRefreshReason {
	fn records_resize_reflow(self) -> bool {
		matches!(self, Self::ResizeReflow)
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
		self.perform(message.into());
		self.perf.flush_canvas_metrics();
		Task::none()
	}

	pub(super) fn perform(&mut self, command: AppCommand) {
		let _span = trace_span!("editor_app.command", command = command_kind(&command)).entered();
		match command {
			AppCommand::Control(message) => self.perform_control(message),
			AppCommand::ReplaceDocument(text) => self.perform_document_replacement(text),
			AppCommand::SelectSidebarTab(tab) => self.perform_sidebar_selection(tab),
			AppCommand::HoverCanvas(target) => {
				self.sidebar.set_hovered_target(target);
			}
			AppCommand::SetCanvasFocus(focused) => {
				self.viewport.canvas_focused = focused;
			}
			AppCommand::SetCanvasScroll(scroll) => {
				self.viewport.canvas_focused = true;
				self.viewport.canvas_scroll = scroll;
			}
			AppCommand::BeginPointerSelection { target, intent } => {
				self.viewport.canvas_focused = true;
				self.sidebar.set_selected_target(target);
				self.perform_editor(EditorIntent::Pointer(intent), EditorDispatchSource::PointerPress);
			}
			AppCommand::Editor { intent, source } => self.perform_editor(intent, source),
			AppCommand::PerfTick => {}
			AppCommand::ObserveCanvasResize(size) => {
				self.handle_canvas_viewport_resized(size);
			}
			AppCommand::FlushResizeReflow => self.perform_resize_reflow(),
			AppCommand::ResizePane(event) => {
				self.shell.chrome.resize(event.split, event.ratio);
			}
		}
	}

	pub(super) fn ensure_scene_ready(&mut self) {
		self.perform_scene_refresh();
	}

	fn perform_control(&mut self, message: ControlsMessage) {
		match message {
			ControlsMessage::LoadPreset(preset) => self.perform_preset_load(preset),
			ControlsMessage::FontSelected(font) => {
				if self.controls.font == font {
					return;
				}
				self.controls.font = font;
				self.perform_scene_config_sync();
			}
			ControlsMessage::ShapingSelected(shaping) => {
				if self.controls.shaping == shaping {
					return;
				}
				self.controls.shaping = shaping;
				self.perform_scene_config_sync();
			}
			ControlsMessage::WrappingSelected(wrapping) => {
				if self.controls.wrapping == wrapping {
					return;
				}
				self.controls.wrapping = wrapping;
				self.perform_scene_config_sync();
			}
			ControlsMessage::FontSizeChanged(font_size) => {
				if (self.controls.font_size - font_size).abs() < f32::EPSILON {
					return;
				}
				self.controls.font_size = font_size;
				self.controls.line_height = self.controls.line_height.max(self.controls.font_size);
				self.perform_scene_config_sync();
			}
			ControlsMessage::LineHeightChanged(line_height) => {
				if (self.controls.line_height - line_height).abs() < f32::EPSILON {
					return;
				}
				self.controls.line_height = line_height;
				self.perform_scene_config_sync();
			}
			ControlsMessage::ShowBaselinesChanged(show_baselines) => {
				if self.controls.show_baselines == show_baselines {
					return;
				}
				self.controls.show_baselines = show_baselines;
				self.perform_scene_refresh();
			}
			ControlsMessage::ShowHitboxesChanged(show_hitboxes) => {
				if self.controls.show_hitboxes == show_hitboxes {
					return;
				}
				self.controls.show_hitboxes = show_hitboxes;
				self.perform_scene_refresh();
			}
		}
	}

	fn perform_preset_load(&mut self, preset: SamplePreset) {
		self.controls.preset = preset;
		if matches!(preset, SamplePreset::Custom) {
			return;
		}

		let started = Instant::now();
		let transition = self.session.execute(
			SessionCommand::ReplaceDocument {
				text: preset.text().to_string(),
				config: self.scene_config(),
			},
			self.derived_scene_consumer_active(),
		);
		self.apply_session_transition(&transition, ApplyPolicy::reset_scroll(SceneRefreshReason::PresetLoaded));
		trace!(
			preset = %preset,
			duration_ms = duration_ms(started.elapsed()),
			text_bytes = self.session.text().len(),
			"preset loaded"
		);
	}

	fn perform_document_replacement(&mut self, text: String) {
		self.controls.preset = SamplePreset::Custom;
		let transition = self.session.execute(
			SessionCommand::ReplaceDocument {
				text,
				config: self.scene_config(),
			},
			self.derived_scene_consumer_active(),
		);
		self.apply_session_transition(&transition, ApplyPolicy::reset_scroll(SceneRefreshReason::TextEdited));
	}

	fn perform_scene_config_sync(&mut self) {
		let transition = self.session.execute(
			SessionCommand::SyncConfig(self.scene_config()),
			self.derived_scene_consumer_active(),
		);
		self.apply_session_transition(
			&transition,
			ApplyPolicy::reset_scroll(SceneRefreshReason::ControlsChanged),
		);
	}

	fn perform_sidebar_selection(&mut self, tab: SidebarTab) {
		if self.sidebar.active_tab == tab {
			return;
		}

		self.sidebar.set_active_tab(tab);
		if matches!(tab, SidebarTab::Inspect | SidebarTab::Perf) {
			self.perform_scene_refresh();
		}
	}

	fn perform_editor(&mut self, intent: EditorIntent, source: EditorDispatchSource) {
		let _span = trace_span!("editor.intent", intent = ?intent, source = ?source).entered();
		let command_started = Instant::now();
		let apply_started = Instant::now();
		let transition = self.session.execute(
			SessionCommand::ApplyEditorIntent(intent),
			self.derived_scene_consumer_active(),
		);
		let apply_elapsed = apply_started.elapsed();
		self.perf.record_editor_apply(apply_elapsed);

		if transition.document_changed() {
			self.controls.preset = SamplePreset::Custom;
		}
		self.apply_session_transition(
			&transition,
			ApplyPolicy::reveal(
				source.reveals_viewport() && transition.view_changed(),
				transition.document_changed().then_some(SceneRefreshReason::TextEdited),
			),
		);

		let command_elapsed = command_started.elapsed();
		self.perf.record_editor_command(command_elapsed);

		let apply_ms = duration_ms(apply_elapsed);
		let command_ms = duration_ms(command_elapsed);

		if command_ms >= 16.7 {
			warn!(
				apply_ms,
				command_ms,
				text_changed = transition.text_changed(),
				view_changed = transition.view_changed(),
				selection_changed = transition.selection_changed(),
				mode_changed = transition.mode_changed(),
				text_bytes = self.session.text().len(),
				"editor command over frame budget"
			);
		} else if command_ms >= 8.0 {
			debug!(
				apply_ms,
				command_ms,
				text_changed = transition.text_changed(),
				view_changed = transition.view_changed(),
				selection_changed = transition.selection_changed(),
				mode_changed = transition.mode_changed(),
				text_bytes = self.session.text().len(),
				"editor command over warning threshold"
			);
		} else {
			trace!(
				apply_ms,
				command_ms,
				text_changed = transition.text_changed(),
				view_changed = transition.view_changed(),
				selection_changed = transition.selection_changed(),
				mode_changed = transition.mode_changed(),
				text_bytes = self.session.text().len(),
				"editor command applied"
			);
		}
	}

	fn perform_resize_reflow(&mut self) {
		let Some(width) = self.viewport.flush_resize() else {
			return;
		};

		let transition = self
			.session
			.execute(SessionCommand::SyncWidth(width), self.derived_scene_consumer_active());
		self.apply_session_transition(
			&transition,
			ApplyPolicy::scene_refresh(SceneRefreshReason::ResizeReflow),
		);
	}

	fn perform_scene_refresh(&mut self) {
		if !self.derived_scene_consumer_active() {
			return;
		}

		let transition = self.session.ensure_scene_materialized();
		self.apply_session_transition(&transition, ApplyPolicy::keep());
	}

	fn handle_canvas_viewport_resized(&mut self, size: Size) {
		let width_changed = self.viewport.observe_resize(size);
		self.viewport
			.clamp_scroll_to_metrics(self.session.snapshot().editor.viewport_metrics);

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

	fn apply_session_transition(&mut self, transition: &SessionTransition, policy: ApplyPolicy) {
		let keep_clamped = policy.scroll_behavior == ScrollBehavior::KeepClamped;
		if !transition.changed() && !policy.reveal_viewport && keep_clamped {
			return;
		}

		if let Some(duration) = transition.width_sync {
			self.perf.record_editor_width_sync(duration);
		}

		let reset_scroll = policy.scroll_behavior == ScrollBehavior::ResetScroll;
		let snapshot = self.session.snapshot();
		let viewport_metrics = snapshot.editor.viewport_metrics;
		let viewport_target = snapshot.editor.editor.viewport_target;
		if let Some(duration) = transition.scene_materialized {
			self.sidebar.sync_after_scene_refresh();
			self.viewport.finish_refresh(viewport_metrics, reset_scroll);

			if let Some(reason) = policy.scene_refresh_reason {
				self.perf.record_scene_build(duration);
				if reason.records_resize_reflow() {
					self.perf.record_resize_reflow(duration);
				}
				if let Some(layout) = self
					.session
					.snapshot()
					.scene
					.as_ref()
					.map(|scene| scene.layout.as_ref())
				{
					log_scene_refresh("scene rebuilt", duration, layout);
				}
			}
		} else if transition.changed() || reset_scroll {
			self.viewport.finish_refresh(viewport_metrics, reset_scroll);
		}

		if policy.reveal_viewport && keep_clamped {
			self.viewport
				.reveal_target_with_metrics(viewport_target, viewport_metrics);
		}
	}

	fn derived_scene_consumer_active(&self) -> bool {
		matches!(self.sidebar.active_tab, SidebarTab::Inspect | SidebarTab::Perf)
			|| self.controls.show_baselines
			|| self.controls.show_hitboxes
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

fn command_kind(command: &AppCommand) -> &'static str {
	match command {
		AppCommand::Control(_) => "control",
		AppCommand::ReplaceDocument(_) => "replace_document",
		AppCommand::SelectSidebarTab(_) => "select_sidebar_tab",
		AppCommand::HoverCanvas(_) => "hover_canvas",
		AppCommand::SetCanvasFocus(_) => "set_canvas_focus",
		AppCommand::SetCanvasScroll(_) => "set_canvas_scroll",
		AppCommand::BeginPointerSelection { .. } => "begin_pointer_selection",
		AppCommand::Editor { .. } => "editor",
		AppCommand::PerfTick => "perf_tick",
		AppCommand::ObserveCanvasResize(_) => "observe_canvas_resize",
		AppCommand::FlushResizeReflow => "flush_resize_reflow",
		AppCommand::ResizePane(_) => "resize_pane",
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
