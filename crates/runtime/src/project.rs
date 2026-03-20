use glorp_api::DocumentStateView;

pub fn document_view_from_state(state: &crate::state::RuntimeState) -> DocumentStateView {
	let (undo_depth, redo_depth) = state.session.history_depths();
	DocumentStateView {
		revisions: state.revisions,
		text_bytes: state.session.text().len(),
		text_lines: state.session.text().lines().count(),
		undo_depth,
		redo_depth,
	}
}

pub(crate) fn ensure_scene_materialized(state: &mut crate::state::RuntimeState) {
	let session = state.session.execute(
		crate::state::SessionRequest::EnsureScene,
		&state.config,
		state.session.layout_width(),
	);

	if let Some(duration) = session.scene_materialized {
		state.perf.record_scene_build(duration.as_secs_f64() * 1000.0);
	}
}
