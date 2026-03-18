use {
	crate::{inspect::inspect_state, state},
	glorp_api::*,
	glorp_editor::SessionSnapshot,
};

pub fn snapshot_from_state(
	state: &mut crate::state::RuntimeState, level: SceneLevel, include_document_text: bool,
) -> GlorpSnapshot {
	if matches!(level, SceneLevel::Materialize) {
		let _ = state.session.execute(
			crate::state::SessionRequest::EnsureScene,
			&state.config,
			state.ui.layout_width,
		);
		if let Some(scene) = state.session.snapshot().scene.as_ref() {
			state.revisions.scene = Some(scene.revision);
		}
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
		perf: PerfStateView {
			scene_builds: state.perf.scene_builds,
			scene_build_millis: state.perf.scene_build_millis,
		},
		ui: UiStateView {
			active_tab: state.ui.active_tab,
			canvas_focused: state.ui.canvas_focused,
			canvas_scroll_x: state.ui.canvas_scroll_x,
			canvas_scroll_y: state.ui.canvas_scroll_y,
			layout_width: state.ui.layout_width,
			viewport_width: state.ui.viewport_width,
			viewport_height: state.ui.viewport_height,
			pane_ratio: state.ui.pane_ratio,
		},
		document_text: include_document_text.then(|| state.session.text().to_owned()),
	}
}

fn editor_view(snapshot: &SessionSnapshot, text: &str) -> EditorStateView {
	let editor = &snapshot.editor;
	EditorStateView {
		mode: state::mode(editor.editor.mode),
		selection: state::selection_range(editor.editor.selection.clone()),
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
			viewport_target: editor.editor.viewport_target.map(|target| LayoutRectView {
				x: target.x,
				y: target.y,
				width: target.width,
				height: target.height,
			}),
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
