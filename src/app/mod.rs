mod headless;
#[cfg(test)]
mod headless_tests;
mod session;
mod sidebar_cache;
mod sidebar_data;
mod state;
#[cfg(test)]
mod tests;
mod update;
mod view;

use {
	self::{
		session::DocumentSession,
		sidebar_cache::SidebarCache,
		state::{ControlsState, ShellState, SidebarState, ViewportState},
	},
	crate::{perf::PerfMonitor, scene::SceneConfig, types::Message},
	iced::Task,
};

pub struct EditorApp {
	session: DocumentSession,
	controls: ControlsState,
	sidebar: SidebarState,
	viewport: ViewportState,
	shell: ShellState,
	perf: PerfMonitor,
	sidebar_cache: SidebarCache,
}

impl EditorApp {
	pub(crate) fn new() -> (Self, Task<Message>) {
		let controls = ControlsState::new();
		let viewport = ViewportState::new(ControlsState::initial_layout_width());
		let session = DocumentSession::new(controls.preset.text(), controls.scene_config(viewport.layout_width));

		(
			Self {
				session,
				controls,
				sidebar: SidebarState::new(),
				viewport,
				shell: ShellState::new(),
				perf: PerfMonitor::default(),
				sidebar_cache: SidebarCache::default(),
			},
			Task::none(),
		)
	}

	fn scene_config(&self) -> SceneConfig {
		self.controls.scene_config(self.viewport.layout_width)
	}

	#[must_use]
	pub fn headless() -> Self {
		Self::new().0
	}
}
