use {
	crate::{inspect::inspect_state, state},
	glorp_api::*,
	glorp_editor::SessionSnapshot,
};

pub fn snapshot_from_state(
	state: &mut crate::state::RuntimeState, level: SceneLevel, include_document_text: bool,
) -> GlorpSnapshot {
	if matches!(level, SceneLevel::Materialize) {
		ensure_scene_materialized(state);
	}

	let snapshot = state.session.snapshot().clone();
	GlorpSnapshot {
		revisions: state.revisions,
		config: state.config.clone(),
		editor: editor_view(&snapshot, state.session.text()),
		scene: match level {
			SceneLevel::Omit => None,
			SceneLevel::IfReady | SceneLevel::Materialize => scene_view(snapshot.scene.as_ref()),
		},
		inspect: inspect_state(state.ui.hovered_target, state.ui.selected_target),
		perf: perf_state_view(state),
		ui: ui_state_view(state),
		document_text: include_document_text.then(|| state.session.text().to_owned()),
	}
}

pub fn selection_view_from_state(state: &crate::state::RuntimeState) -> SelectionStateView {
	let editor = &state.session.snapshot().editor;
	let text = state.session.text();
	let selection = editor.editor.selection.as_ref();
	let range = state::selection_range(selection);
	SelectionStateView {
		mode: state::mode(editor.editor.mode),
		selected_text: selection.and_then(|selection| text.get(selection.clone()).map(ToOwned::to_owned)),
		range,
		selection_head: state::selection_head(&editor.editor),
		pointer_anchor: state::pointer_anchor(&editor.editor),
		viewport_target: editor.editor.viewport_target.map(layout_rect_view),
	}
}

pub fn inspect_details_view_from_state(
	state: &mut crate::state::RuntimeState, target: Option<CanvasTarget>,
) -> InspectDetailsView {
	ensure_scene_materialized(state);

	let active_target = target.or(state.ui.selected_target).or(state.ui.hovered_target);
	let scene = state.session.snapshot().scene.as_ref();
	let (warnings, interaction_details, scene) = scene.map_or_else(
		|| (Vec::new(), "derived scene unavailable".to_owned(), None),
		|scene| {
			(
				scene.layout.warnings.to_vec(),
				scene
					.layout
					.target_details(active_target)
					.as_deref()
					.unwrap_or("hover a run or cluster for details")
					.to_owned(),
				Some(InspectSceneView {
					revision: scene.revision,
					run_count: scene.layout.runs.len(),
					cluster_count: scene.layout.cluster_count,
				}),
			)
		},
	);

	InspectDetailsView {
		hovered_target: state.ui.hovered_target,
		selected_target: state.ui.selected_target,
		active_target,
		warnings,
		interaction_details,
		scene,
	}
}

pub fn perf_dashboard_view_from_state(state: &mut crate::state::RuntimeState) -> PerfDashboardView {
	ensure_scene_materialized(state);

	let snapshot = state.session.snapshot();
	let scene = snapshot.scene.as_ref();
	PerfDashboardView {
		overview: PerfOverviewView {
			editor_mode: state::mode(snapshot.mode()),
			editor_bytes: snapshot.editor_bytes(),
			text_lines: state.session.text().lines().count().max(1),
			layout_width: state.ui.layout_width,
			scene_ready: scene.is_some(),
			scene_revision: scene.map(|scene| scene.revision),
			scene_width: scene.map_or(0.0, |scene| scene.layout.measured_width),
			scene_height: scene.map_or(0.0, |scene| scene.layout.measured_height),
			run_count: scene.map_or(0, |scene| scene.layout.runs.len()),
			cluster_count: scene.map_or(0, |scene| scene.layout.cluster_count),
			warning_count: scene.map_or(0, |scene| scene.layout.warnings.len()),
		},
		metrics: vec![PerfMetricSummaryView {
			label: "scene.build".to_owned(),
			total_samples: state.perf.scene_build.total_samples,
			total_millis: state.perf.scene_build.total_millis,
			last_millis: state.perf.scene_build.last_millis,
			avg_millis: state.perf.scene_build.average_millis(),
		}],
	}
}

pub fn ui_state_view(state: &crate::state::RuntimeState) -> UiStateView {
	UiStateView {
		active_tab: state.ui.active_tab,
		canvas_focused: state.ui.canvas_focused,
		canvas_scroll_x: state.ui.canvas_scroll_x,
		canvas_scroll_y: state.ui.canvas_scroll_y,
		layout_width: state.ui.layout_width,
		viewport_width: state.ui.viewport_width,
		viewport_height: state.ui.viewport_height,
		pane_ratio: state.ui.pane_ratio,
	}
}

pub fn perf_state_view(state: &crate::state::RuntimeState) -> PerfStateView {
	PerfStateView {
		scene_builds: state.perf.scene_build.total_samples as usize,
		scene_build_millis: state.perf.scene_build.total_millis,
	}
}

fn ensure_scene_materialized(state: &mut crate::state::RuntimeState) {
	let session = state.session.execute(
		crate::state::SessionRequest::EnsureScene,
		&state.config,
		state.ui.layout_width,
	);
	if let Some(scene) = state.session.snapshot().scene.as_ref() {
		state.revisions.scene = Some(scene.revision);
	}
	if let Some(duration) = session.delta.scene_materialized {
		state.perf.record_scene_build(duration.as_secs_f64() * 1000.0);
	}
}

fn editor_view(snapshot: &SessionSnapshot, text: &str) -> EditorStateView {
	let editor = &snapshot.editor;
	EditorStateView {
		mode: state::mode(editor.editor.mode),
		selection: state::selection_range(editor.editor.selection.as_ref()),
		selection_head: state::selection_head(&editor.editor),
		pointer_anchor: state::pointer_anchor(&editor.editor),
		text_bytes: editor.editor_bytes,
		text_lines: text.lines().count(),
		undo_depth: editor.undo_depth,
		redo_depth: editor.redo_depth,
		viewport: EditorViewportView {
			wrapping: editor.viewport_metrics.wrapping,
			measured_width: editor.viewport_metrics.measured_width,
			measured_height: editor.viewport_metrics.measured_height,
			viewport_target: editor.editor.viewport_target.map(layout_rect_view),
		},
	}
}

fn scene_view(scene: Option<&glorp_editor::ScenePresentation>) -> Option<SceneStateView> {
	scene.map(|scene| SceneStateView {
		revision: scene.revision,
		measured_width: scene.layout.measured_width,
		measured_height: scene.layout.measured_height,
		run_count: scene.layout.runs.len(),
		cluster_count: scene.layout.cluster_count,
	})
}

fn layout_rect_view(target: glorp_editor::LayoutRect) -> LayoutRectView {
	LayoutRectView {
		x: target.x,
		y: target.y,
		width: target.width,
		height: target.height,
	}
}
