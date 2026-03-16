mod headless;
#[cfg(test)]
mod headless_tests;
mod session;
mod state;
#[cfg(test)]
mod tests;
mod update;
mod view;

use {
	self::{
		session::SceneSession,
		state::{ControlsState, ShellState, SidebarState, ViewportState},
	},
	crate::{perf::PerfMonitor, scene::SceneConfig, types::Message},
	iced::Task,
};

pub struct Playground {
	session: SceneSession,
	scene_dirty: bool,
	deferred_resize_reflow: bool,
	controls: ControlsState,
	sidebar: SidebarState,
	viewport: ViewportState,
	shell: ShellState,
	perf: PerfMonitor,
}

impl Playground {
	pub(crate) fn new() -> (Self, Task<Message>) {
		let controls = ControlsState::new();
		let viewport = ViewportState::new(ControlsState::initial_layout_width());
		let session = SceneSession::new(controls.preset.text(), controls.scene_config(viewport.layout_width));

		(
			Self {
				session,
				scene_dirty: false,
				deferred_resize_reflow: false,
				controls,
				sidebar: SidebarState::new(),
				viewport,
				shell: ShellState::new(),
				perf: PerfMonitor::default(),
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
