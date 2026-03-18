use {
	crate::state,
	glorp_api::{EditorStateView, EditorViewportView, LayoutRectView},
	std::ops::Range,
};

pub fn editor_view_from_state(state: &crate::state::RuntimeState) -> EditorStateView {
	let snapshot = state.session.snapshot();
	let editor = &snapshot.editor;
	let selection = editor.editor.selection.as_ref();

	EditorStateView {
		revisions: state.revisions,
		mode: state::mode(editor.editor.mode),
		selection: selection.map(state::text_range),
		selected_text: selected_text(selection, state.session.text()),
		selection_head: state::selection_head(&editor.editor),
		pointer_anchor: state::pointer_anchor(&editor.editor),
		text_bytes: editor.editor_bytes,
		text_lines: state.session.text().lines().count(),
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

pub(crate) fn ensure_scene_materialized(state: &mut crate::state::RuntimeState) {
	let session = state.session.execute(
		crate::state::SessionRequest::EnsureScene,
		&state.config,
		state.ui.layout_width,
	);

	if let Some(duration) = session.scene_materialized {
		state.perf.record_scene_build(duration.as_secs_f64() * 1000.0);
	}
}

fn selected_text(selection: Option<&Range<usize>>, text: &str) -> Option<String> {
	selection
		.and_then(|selection| text.get(selection.start..selection.end))
		.map(str::to_owned)
}

const fn layout_rect_view(target: glorp_editor::LayoutRect) -> LayoutRectView {
	LayoutRectView {
		x: target.x,
		y: target.y,
		width: target.width,
		height: target.height,
	}
}
