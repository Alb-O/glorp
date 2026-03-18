use glorp_api::InspectStateView;

#[must_use]
pub const fn inspect_state(
	hovered_target: Option<glorp_api::CanvasTarget>, selected_target: Option<glorp_api::CanvasTarget>,
) -> InspectStateView {
	InspectStateView {
		hovered_target,
		selected_target,
	}
}
