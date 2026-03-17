use {
	super::{
		AppModel,
		state::{EditorDispatchSource, RESIZE_REFLOW_INTERVAL},
	},
	crate::{
		editor::{EditorIntent, EditorPointerIntent},
		overlay::LayoutRect,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScrollBehavior {
	KeepClamped,
	ResetScroll,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SceneRefresh {
	reason: Option<SceneRefreshReason>,
	duration: Duration,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct AppEffects {
	pub(super) text_changed: bool,
	pub(super) view_changed: bool,
	pub(super) selection_changed: bool,
	pub(super) mode_changed: bool,
	pub(super) viewport_target: Option<LayoutRect>,
	pub(super) width_sync: Option<Duration>,
	scroll_behavior: ScrollBehavior,
	reveal_viewport: bool,
	scene_refresh: Option<SceneRefresh>,
}

impl Default for ScrollBehavior {
	fn default() -> Self {
		Self::KeepClamped
	}
}

impl AppEffects {
	fn changed(&self) -> bool {
		self.text_changed
			|| self.view_changed
			|| self.selection_changed
			|| self.mode_changed
			|| self.width_sync.is_some()
			|| self.scene_refresh.is_some()
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
		let _ = self.perform(message.into());
		self.perf.flush_canvas_metrics();
		Task::none()
	}

	pub(super) fn perform(&mut self, command: AppCommand) -> AppEffects {
		let _span = trace_span!("editor_app.command", command = command_kind(&command)).entered();
		match command {
			AppCommand::Control(message) => self.perform_control(message),
			AppCommand::ReplaceDocument(text) => self.perform_document_replacement(text),
			AppCommand::SelectSidebarTab(tab) => self.perform_sidebar_selection(tab),
			AppCommand::HoverCanvas(target) => {
				self.sidebar.set_hovered_target(target);
				AppEffects::default()
			}
			AppCommand::SetCanvasFocus(focused) => {
				self.viewport.canvas_focused = focused;
				AppEffects::default()
			}
			AppCommand::SetCanvasScroll(scroll) => {
				self.viewport.canvas_focused = true;
				self.viewport.canvas_scroll = scroll;
				AppEffects::default()
			}
			AppCommand::BeginPointerSelection { target, intent } => {
				self.viewport.canvas_focused = true;
				self.sidebar.set_selected_target(target);
				self.perform_editor(EditorIntent::Pointer(intent), EditorDispatchSource::PointerPress)
			}
			AppCommand::Editor { intent, source } => self.perform_editor(intent, source),
			AppCommand::PerfTick => AppEffects::default(),
			AppCommand::ObserveCanvasResize(size) => {
				self.handle_canvas_viewport_resized(size);
				AppEffects::default()
			}
			AppCommand::FlushResizeReflow => self.perform_resize_reflow(),
			AppCommand::ResizePane(event) => {
				self.shell.chrome.resize(event.split, event.ratio);
				AppEffects::default()
			}
		}
	}

	pub(super) fn ensure_scene_ready(&mut self) -> AppEffects {
		self.perform_scene_refresh()
	}

	fn finalize_effects(&mut self, effects: AppEffects) -> AppEffects {
		self.apply_effects(&effects);
		effects
	}

	fn perform_control(&mut self, message: ControlsMessage) -> AppEffects {
		match message {
			ControlsMessage::LoadPreset(preset) => self.perform_preset_load(preset),
			ControlsMessage::FontSelected(font) => {
				if self.controls.font == font {
					return AppEffects::default();
				}
				self.controls.font = font;
				self.perform_scene_config_sync()
			}
			ControlsMessage::ShapingSelected(shaping) => {
				if self.controls.shaping == shaping {
					return AppEffects::default();
				}
				self.controls.shaping = shaping;
				self.perform_scene_config_sync()
			}
			ControlsMessage::WrappingSelected(wrapping) => {
				if self.controls.wrapping == wrapping {
					return AppEffects::default();
				}
				self.controls.wrapping = wrapping;
				self.perform_scene_config_sync()
			}
			ControlsMessage::FontSizeChanged(font_size) => {
				if (self.controls.font_size - font_size).abs() < f32::EPSILON {
					return AppEffects::default();
				}
				self.controls.font_size = font_size;
				self.controls.line_height = self.controls.line_height.max(self.controls.font_size);
				self.perform_scene_config_sync()
			}
			ControlsMessage::LineHeightChanged(line_height) => {
				if (self.controls.line_height - line_height).abs() < f32::EPSILON {
					return AppEffects::default();
				}
				self.controls.line_height = line_height;
				self.perform_scene_config_sync()
			}
			ControlsMessage::ShowBaselinesChanged(show_baselines) => {
				if self.controls.show_baselines == show_baselines {
					return AppEffects::default();
				}
				self.controls.show_baselines = show_baselines;
				self.perform_scene_refresh()
			}
			ControlsMessage::ShowHitboxesChanged(show_hitboxes) => {
				if self.controls.show_hitboxes == show_hitboxes {
					return AppEffects::default();
				}
				self.controls.show_hitboxes = show_hitboxes;
				self.perform_scene_refresh()
			}
		}
	}

	fn perform_preset_load(&mut self, preset: SamplePreset) -> AppEffects {
		self.controls.preset = preset;
		if matches!(preset, SamplePreset::Custom) {
			return AppEffects::default();
		}

		let started = Instant::now();
		self.session.replace_document(preset.text(), self.scene_config());
		let effects = self.replace_document_effects(SceneRefreshReason::PresetLoaded);
		trace!(
			preset = %preset,
			duration_ms = duration_ms(started.elapsed()),
			text_bytes = self.session.text().len(),
			"preset loaded"
		);
		effects
	}

	fn perform_document_replacement(&mut self, text: String) -> AppEffects {
		self.controls.preset = SamplePreset::Custom;
		self.session.replace_document(&text, self.scene_config());
		self.replace_document_effects(SceneRefreshReason::TextEdited)
	}

	fn perform_scene_config_sync(&mut self) -> AppEffects {
		if !self.session.sync_config(self.scene_config()) {
			return AppEffects::default();
		}

		let scene_refresh = self.ensure_scene_if_demanded(Some(SceneRefreshReason::ControlsChanged));
		self.finalize_effects(AppEffects {
			view_changed: true,
			viewport_target: self.session.viewport_target(),
			scroll_behavior: ScrollBehavior::ResetScroll,
			scene_refresh,
			..AppEffects::default()
		})
	}

	fn perform_sidebar_selection(&mut self, tab: SidebarTab) -> AppEffects {
		if self.sidebar.active_tab == tab {
			return AppEffects::default();
		}

		self.sidebar.set_active_tab(tab);
		matches!(tab, SidebarTab::Inspect | SidebarTab::Perf)
			.then(|| self.perform_scene_refresh())
			.unwrap_or_default()
	}

	fn perform_editor(&mut self, intent: EditorIntent, source: EditorDispatchSource) -> AppEffects {
		let _span = trace_span!("editor.intent", intent = ?intent, source = ?source).entered();
		let command_started = Instant::now();
		let apply_started = Instant::now();
		let outcome = self.session.apply_editor_intent(intent);
		let apply_elapsed = apply_started.elapsed();
		self.perf.record_editor_apply(apply_elapsed);

		let mut effects = AppEffects {
			text_changed: outcome.document_changed(),
			view_changed: outcome.view_changed,
			selection_changed: outcome.selection_changed,
			mode_changed: outcome.mode_changed,
			viewport_target: outcome.viewport_target,
			scroll_behavior: ScrollBehavior::KeepClamped,
			reveal_viewport: source.reveals_viewport() && outcome.view_changed,
			..AppEffects::default()
		};
		if effects.text_changed {
			self.controls.preset = SamplePreset::Custom;
			effects.scene_refresh = self.ensure_scene_if_demanded(Some(SceneRefreshReason::TextEdited));
		}
		self.apply_effects(&effects);

		let command_elapsed = command_started.elapsed();
		self.perf.record_editor_command(command_elapsed);

		let apply_ms = duration_ms(apply_elapsed);
		let command_ms = duration_ms(command_elapsed);

		if command_ms >= 16.7 {
			warn!(
				apply_ms,
				command_ms,
				text_changed = effects.text_changed,
				view_changed = effects.view_changed,
				selection_changed = effects.selection_changed,
				mode_changed = effects.mode_changed,
				text_bytes = self.session.text().len(),
				"editor command over frame budget"
			);
		} else if command_ms >= 8.0 {
			debug!(
				apply_ms,
				command_ms,
				text_changed = effects.text_changed,
				view_changed = effects.view_changed,
				selection_changed = effects.selection_changed,
				mode_changed = effects.mode_changed,
				text_bytes = self.session.text().len(),
				"editor command over warning threshold"
			);
		} else {
			trace!(
				apply_ms,
				command_ms,
				text_changed = effects.text_changed,
				view_changed = effects.view_changed,
				selection_changed = effects.selection_changed,
				mode_changed = effects.mode_changed,
				text_bytes = self.session.text().len(),
				"editor command applied"
			);
		}

		effects
	}

	fn perform_resize_reflow(&mut self) -> AppEffects {
		let Some(width) = self.viewport.flush_resize() else {
			return AppEffects::default();
		};
		let Some(duration) = self.session.sync_width(width) else {
			return AppEffects::default();
		};

		let scene_refresh = self.ensure_scene_if_demanded(Some(SceneRefreshReason::ResizeReflow));
		self.finalize_effects(AppEffects {
			view_changed: true,
			viewport_target: self.session.viewport_target(),
			width_sync: Some(duration),
			scene_refresh,
			..AppEffects::default()
		})
	}

	fn perform_scene_refresh(&mut self) -> AppEffects {
		let scene_refresh = self.ensure_scene_if_demanded(None);
		self.finalize_effects(AppEffects {
			scene_refresh,
			..AppEffects::default()
		})
	}

	fn replace_document_effects(&mut self, reason: SceneRefreshReason) -> AppEffects {
		let scene_refresh = self.ensure_scene_if_demanded(Some(reason));
		self.finalize_effects(AppEffects {
			text_changed: true,
			view_changed: true,
			selection_changed: true,
			mode_changed: true,
			viewport_target: self.session.viewport_target(),
			scroll_behavior: ScrollBehavior::ResetScroll,
			scene_refresh,
			..AppEffects::default()
		})
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

	fn ensure_scene_if_demanded(&mut self, reason: Option<SceneRefreshReason>) -> Option<SceneRefresh> {
		self.derived_scene_consumer_active()
			.then(|| self.session.ensure_scene())
			.flatten()
			.map(|duration| SceneRefresh { reason, duration })
	}

	fn apply_effects(&mut self, effects: &AppEffects) {
		if !effects.changed() && !effects.reveal_viewport && effects.scroll_behavior == ScrollBehavior::KeepClamped {
			return;
		}

		if let Some(duration) = effects.width_sync {
			self.perf.record_editor_width_sync(duration);
		}

		let reset_scroll = matches!(effects.scroll_behavior, ScrollBehavior::ResetScroll);
		if let Some(SceneRefresh { reason, duration }) = effects.scene_refresh {
			if let Some(scene) = self.session.snapshot().scene.as_ref() {
				self.sidebar.sync_after_scene_refresh();
				self.viewport.finish_scene_refresh(scene.layout.as_ref(), reset_scroll);

				if let Some(reason) = reason {
					self.perf.record_scene_build(duration);
					if reason.records_resize_reflow() {
						self.perf.record_resize_reflow(duration);
					}
					log_scene_refresh("scene rebuilt", duration, scene.layout.as_ref());
				}
			}
		} else if effects.changed() || reset_scroll {
			self.viewport
				.finish_editor_refresh(self.session.viewport_metrics(), reset_scroll);
		}

		if effects.reveal_viewport && matches!(effects.scroll_behavior, ScrollBehavior::KeepClamped) {
			self.viewport
				.reveal_target_with_metrics(effects.viewport_target, self.session.viewport_metrics());
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
