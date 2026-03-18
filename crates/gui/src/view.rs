use crate::GuiPresentation;

#[must_use]
pub fn describe(presentation: &GuiPresentation) -> String {
	format!(
		"tab={} scroll=({}, {})",
		match presentation.snapshot.ui.active_tab {
			glorp_api::SidebarTab::Controls => "controls",
			glorp_api::SidebarTab::Inspect => "inspect",
			glorp_api::SidebarTab::Perf => "perf",
		},
		presentation.snapshot.ui.canvas_scroll_x,
		presentation.snapshot.ui.canvas_scroll_y
	)
}
