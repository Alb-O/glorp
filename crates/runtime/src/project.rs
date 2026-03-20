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
