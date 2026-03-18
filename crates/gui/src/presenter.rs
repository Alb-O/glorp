use glorp_api::GlorpSnapshot;

#[derive(Debug, Clone, PartialEq)]
pub struct GuiPresentation {
	pub snapshot: GlorpSnapshot,
}

impl GuiPresentation {
	#[must_use]
	pub const fn active_tab(&self) -> glorp_api::SidebarTab {
		self.snapshot.ui.active_tab
	}
}
