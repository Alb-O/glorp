use glorp_api::SidebarTab;

#[must_use]
pub const fn is_inspect(tab: SidebarTab) -> bool {
	matches!(tab, SidebarTab::Inspect)
}
