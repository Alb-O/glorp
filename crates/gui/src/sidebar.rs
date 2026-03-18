use glorp_api::SidebarTab;

#[must_use]
pub fn is_inspect(tab: SidebarTab) -> bool {
	matches!(tab, SidebarTab::Inspect)
}
