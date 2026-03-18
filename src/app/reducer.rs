use {
	super::{
		action::{AppAction, SceneRefreshReason, ScrollBehavior, SessionEffect, SessionUiPolicy},
		session::{SceneDemand, SessionFeedback, SessionRequest},
		state::AppState,
	},
	crate::{
		editor::EditorIntent,
		types::{ControlsMessage, SamplePreset, SidebarTab},
	},
};

pub(super) fn reduce(state: &mut AppState, action: AppAction) -> Option<SessionEffect> {
	match action {
		AppAction::Control(message) => reduce_control(state, message),
		AppAction::ReplaceDocument(text) => {
			state.controls.preset = SamplePreset::Custom;
			Some(session_effect(
				state,
				SessionRequest::ReplaceDocument {
					text,
					config: state.scene_config(),
				},
				SessionUiPolicy::reset_scroll(SceneRefreshReason::TextEdited),
			))
		}
		AppAction::SelectSidebarTab(tab) => {
			if state.sidebar.active_tab == tab {
				return None;
			}

			state.sidebar.set_active_tab(tab);
			if matches!(tab, SidebarTab::Inspect | SidebarTab::Perf) {
				ensure_scene_effect(state)
			} else {
				None
			}
		}
		AppAction::HoverCanvas(target) => {
			state.sidebar.set_hovered_target(target);
			None
		}
		AppAction::SetCanvasFocus(focused) => {
			state.viewport.canvas_focused = focused;
			None
		}
		AppAction::SetCanvasScroll(scroll) => {
			state.viewport.canvas_focused = true;
			state.viewport.canvas_scroll = scroll;
			None
		}
		AppAction::BeginPointerSelection { target, intent } => {
			state.viewport.canvas_focused = true;
			state.sidebar.set_selected_target(target);
			Some(session_effect(
				state,
				SessionRequest::ApplyEditorIntent(EditorIntent::Pointer(intent)),
				SessionUiPolicy::reveal(true, None),
			))
		}
		AppAction::Editor { intent, source } => Some(session_effect(
			state,
			SessionRequest::ApplyEditorIntent(intent),
			SessionUiPolicy::reveal(source.reveals_viewport(), Some(SceneRefreshReason::TextEdited)),
		)),
		AppAction::PerfTick => None,
		AppAction::ObserveCanvasResize(size) => {
			state.viewport.observe_resize(size);
			state.viewport.clamp_scroll_to_metrics(state.editor_metrics);
			None
		}
		AppAction::FlushResizeReflow => state.viewport.flush_resize().map(|width| {
			session_effect(
				state,
				SessionRequest::SyncWidth(width),
				SessionUiPolicy::scene_refresh(SceneRefreshReason::ResizeReflow),
			)
		}),
		AppAction::ResizePane(event) => {
			state.shell.chrome.resize(event.split, event.ratio);
			None
		}
		AppAction::EnsureScene => ensure_scene_effect(state),
	}
}

fn reduce_control(state: &mut AppState, message: ControlsMessage) -> Option<SessionEffect> {
	match message {
		ControlsMessage::LoadPreset(preset) => {
			state.controls.preset = preset;
			if matches!(preset, SamplePreset::Custom) {
				return None;
			}

			Some(session_effect(
				state,
				SessionRequest::ReplaceDocument {
					text: preset.text().to_string(),
					config: state.scene_config(),
				},
				SessionUiPolicy::reset_scroll(SceneRefreshReason::PresetLoaded),
			))
		}
		ControlsMessage::FontSelected(font) => {
			if state.controls.font == font {
				return None;
			}
			state.controls.font = font;
			Some(sync_config_effect(state))
		}
		ControlsMessage::ShapingSelected(shaping) => {
			if state.controls.shaping == shaping {
				return None;
			}
			state.controls.shaping = shaping;
			Some(sync_config_effect(state))
		}
		ControlsMessage::WrappingSelected(wrapping) => {
			if state.controls.wrapping == wrapping {
				return None;
			}
			state.controls.wrapping = wrapping;
			Some(sync_config_effect(state))
		}
		ControlsMessage::FontSizeChanged(font_size) => {
			if (state.controls.font_size - font_size).abs() < f32::EPSILON {
				return None;
			}
			state.controls.font_size = font_size;
			state.controls.line_height = state.controls.line_height.max(state.controls.font_size);
			Some(sync_config_effect(state))
		}
		ControlsMessage::LineHeightChanged(line_height) => {
			if (state.controls.line_height - line_height).abs() < f32::EPSILON {
				return None;
			}
			state.controls.line_height = line_height;
			Some(sync_config_effect(state))
		}
		ControlsMessage::ShowBaselinesChanged(show_baselines) => {
			if state.controls.show_baselines == show_baselines {
				return None;
			}
			state.controls.show_baselines = show_baselines;
			ensure_scene_effect(state)
		}
		ControlsMessage::ShowHitboxesChanged(show_hitboxes) => {
			if state.controls.show_hitboxes == show_hitboxes {
				return None;
			}
			state.controls.show_hitboxes = show_hitboxes;
			ensure_scene_effect(state)
		}
	}
}

fn sync_config_effect(state: &AppState) -> SessionEffect {
	session_effect(
		state,
		SessionRequest::SyncConfig(state.scene_config()),
		SessionUiPolicy::reset_scroll(SceneRefreshReason::ControlsChanged),
	)
}

fn ensure_scene_effect(state: &AppState) -> Option<SessionEffect> {
	match scene_demand(state) {
		SceneDemand::HotPathOnly => None,
		demand @ SceneDemand::DerivedScene => Some(SessionEffect {
			request: SessionRequest::EnsureScene,
			demand,
			policy: SessionUiPolicy::keep(),
		}),
	}
}

fn session_effect(state: &AppState, request: SessionRequest, policy: SessionUiPolicy) -> SessionEffect {
	SessionEffect {
		request,
		demand: scene_demand(state),
		policy,
	}
}

fn scene_demand(state: &AppState) -> SceneDemand {
	if matches!(state.sidebar.active_tab, SidebarTab::Inspect | SidebarTab::Perf)
		|| state.controls.show_baselines
		|| state.controls.show_hitboxes
	{
		SceneDemand::DerivedScene
	} else {
		SceneDemand::HotPathOnly
	}
}

pub(super) fn apply_session_feedback(state: &mut AppState, feedback: &SessionFeedback, policy: SessionUiPolicy) {
	let keep_clamped = policy.scroll_behavior == ScrollBehavior::KeepClamped;
	if !feedback.delta.changed() && !policy.reveal_viewport && keep_clamped {
		return;
	}

	if feedback.delta.document_changed() {
		state.controls.preset = SamplePreset::Custom;
	}

	let metrics = feedback.snapshot.editor.viewport_metrics;
	state.editor_metrics = metrics;

	let reset_scroll = policy.scroll_behavior == ScrollBehavior::ResetScroll;
	if feedback.delta.scene_materialized.is_some() {
		state.sidebar.sync_after_scene_refresh();
		state.viewport.finish_refresh(metrics, reset_scroll);
	} else if feedback.delta.changed() || reset_scroll {
		state.viewport.finish_refresh(metrics, reset_scroll);
	}

	if policy.reveal_viewport && feedback.delta.view_changed() && keep_clamped {
		state
			.viewport
			.reveal_target_with_metrics(feedback.snapshot.editor.editor.viewport_target, metrics);
	}
}
