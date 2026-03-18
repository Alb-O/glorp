use {
	super::{
		action::{AppAction, SceneRefreshReason, SessionEffect, SessionUiPolicy},
		reducer,
		session::{DocumentSession, SessionDelta, SessionFeedback, SessionRequest},
		sidebar_cache::SidebarCache,
		state::{AppState, ControlsState, RESIZE_REFLOW_INTERVAL, ViewportState},
	},
	crate::{
		perf::PerfMonitor,
		telemetry::duration_ms,
		types::{Message, PerfMessage, SidebarTab, ViewportMessage},
	},
	iced::{Subscription, time},
	std::time::{Duration, Instant},
	tracing::{debug, trace, trace_span, warn},
};

pub(super) struct AppStore {
	pub(super) state: AppState,
	pub(super) session: DocumentSession,
	pub(super) perf: PerfMonitor,
	pub(super) sidebar_cache: SidebarCache,
}

impl AppStore {
	pub(super) fn new() -> Self {
		let controls = ControlsState::new();
		let viewport = ViewportState::new(ControlsState::initial_layout_width());
		let session = DocumentSession::new(controls.preset.text(), controls.scene_config(viewport.layout_width));
		let editor_metrics = session.snapshot().editor.viewport_metrics;

		Self {
			state: AppState::new(controls, viewport, editor_metrics),
			session,
			perf: PerfMonitor::default(),
			sidebar_cache: SidebarCache::default(),
		}
	}

	pub(super) fn subscription(&self) -> Subscription<Message> {
		let perf = self.state.sidebar.active_tab == SidebarTab::Perf;
		let resize = self.state.viewport.resize_coalescer.has_pending();

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

	pub(super) fn dispatch(&mut self, action: AppAction) {
		let is_editor_command = action.is_editor_command();
		let command_started = Instant::now();
		let _span = trace_span!("editor_app.action", action = action_kind(&action)).entered();
		let delta = self.process_action(action);

		if is_editor_command {
			let elapsed = command_started.elapsed();
			self.perf.record_editor_command(elapsed);
			log_editor_command(elapsed, delta.as_ref(), self.session.text().len());
		}
	}

	pub(super) fn ensure_scene_ready(&mut self) {
		let _ = self.process_action(AppAction::EnsureScene);
	}

	fn process_action(&mut self, action: AppAction) -> Option<SessionDelta> {
		reducer::reduce(&mut self.state, action).map(|effect| self.execute_effect(effect))
	}

	fn execute_effect(&mut self, effect: SessionEffect) -> SessionDelta {
		let SessionEffect {
			request,
			demand,
			policy,
		} = effect;
		let is_editor_apply = matches!(request, SessionRequest::ApplyEditorIntent(_));
		let started = Instant::now();
		let feedback = self.session.execute(request, demand);
		let elapsed = started.elapsed();

		if is_editor_apply {
			self.perf.record_editor_apply(elapsed);
			log_editor_apply(elapsed, &feedback.delta, self.session.text().len());
		}

		self.record_feedback_metrics(&feedback, policy);
		reducer::apply_session_feedback(&mut self.state, &feedback, policy);
		feedback.delta
	}

	fn record_feedback_metrics(&mut self, feedback: &SessionFeedback, policy: SessionUiPolicy) {
		if let Some(duration) = feedback.delta.width_sync {
			self.perf.record_editor_width_sync(duration);
		}

		let scene_reason = policy
			.scene_refresh_reason
			.filter(|reason| *reason != SceneRefreshReason::TextEdited || feedback.delta.document_changed());

		if let Some(duration) = feedback.delta.scene_materialized
			&& let Some(reason) = scene_reason
		{
			self.perf.record_scene_build(duration);
			if reason.records_resize_reflow() {
				self.perf.record_resize_reflow(duration);
			}
			if let Some(layout) = feedback.snapshot.scene.as_ref().map(|scene| scene.layout.as_ref()) {
				log_scene_refresh("scene rebuilt", duration, layout);
			}
		}
	}
}

fn action_kind(action: &AppAction) -> &'static str {
	match action {
		AppAction::Control(_) => "control",
		AppAction::ReplaceDocument(_) => "replace_document",
		AppAction::SelectSidebarTab(_) => "select_sidebar_tab",
		AppAction::HoverCanvas(_) => "hover_canvas",
		AppAction::SetCanvasFocus(_) => "set_canvas_focus",
		AppAction::SetCanvasScroll(_) => "set_canvas_scroll",
		AppAction::BeginPointerSelection { .. } => "begin_pointer_selection",
		AppAction::Editor { .. } => "editor",
		AppAction::PerfTick => "perf_tick",
		AppAction::ObserveCanvasResize(_) => "observe_canvas_resize",
		AppAction::FlushResizeReflow => "flush_resize_reflow",
		AppAction::ResizePane(_) => "resize_pane",
		AppAction::EnsureScene => "ensure_scene",
	}
}

fn log_editor_apply(elapsed: Duration, delta: &SessionDelta, text_bytes: usize) {
	let elapsed_ms = duration_ms(elapsed);
	if elapsed_ms >= 16.7 {
		warn!(
			apply_ms = elapsed_ms,
			text_changed = delta.text_changed(),
			view_changed = delta.view_changed(),
			selection_changed = delta.selection_changed(),
			mode_changed = delta.mode_changed(),
			text_bytes,
			"editor apply over frame budget"
		);
	} else if elapsed_ms >= 8.0 {
		debug!(
			apply_ms = elapsed_ms,
			text_changed = delta.text_changed(),
			view_changed = delta.view_changed(),
			selection_changed = delta.selection_changed(),
			mode_changed = delta.mode_changed(),
			text_bytes,
			"editor apply over warning threshold"
		);
	} else {
		trace!(
			apply_ms = elapsed_ms,
			text_changed = delta.text_changed(),
			view_changed = delta.view_changed(),
			selection_changed = delta.selection_changed(),
			mode_changed = delta.mode_changed(),
			text_bytes,
			"editor apply"
		);
	}
}

fn log_editor_command(elapsed: Duration, delta: Option<&SessionDelta>, text_bytes: usize) {
	let elapsed_ms = duration_ms(elapsed);
	let text_changed = delta.is_some_and(SessionDelta::text_changed);
	let view_changed = delta.is_some_and(SessionDelta::view_changed);
	let selection_changed = delta.is_some_and(SessionDelta::selection_changed);
	let mode_changed = delta.is_some_and(SessionDelta::mode_changed);

	if elapsed_ms >= 16.7 {
		warn!(
			command_ms = elapsed_ms,
			text_changed, view_changed, selection_changed, mode_changed, text_bytes, "editor command over frame budget"
		);
	} else if elapsed_ms >= 8.0 {
		debug!(
			command_ms = elapsed_ms,
			text_changed,
			view_changed,
			selection_changed,
			mode_changed,
			text_bytes,
			"editor command over warning threshold"
		);
	} else {
		trace!(
			command_ms = elapsed_ms,
			text_changed, view_changed, selection_changed, mode_changed, text_bytes, "editor command applied"
		);
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
