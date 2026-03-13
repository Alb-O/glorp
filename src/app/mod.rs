mod headless;
#[cfg(test)]
mod headless_tests;
mod session;
mod state;
#[cfg(test)]
mod tests;
mod update;
mod view;

use iced::Task;

use crate::perf::PerfMonitor;
use crate::scene::SceneConfig;
use crate::types::Message;

use self::session::SceneSession;
use self::state::{ControlsState, ShellState, SidebarState, ViewportState};

pub struct Playground {
	session: SceneSession,
	controls: ControlsState,
	sidebar: SidebarState,
	viewport: ViewportState,
	shell: ShellState,
	perf: PerfMonitor,
}

impl Playground {
	pub(crate) fn new() -> (Self, Task<Message>) {
		let controls = ControlsState::new();
		let viewport = ViewportState::new(controls.initial_layout_width());
		let session = SceneSession::new(controls.preset.text(), controls.scene_config(viewport.layout_width));

		(
			Self {
				session,
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

	pub fn headless() -> Self {
		Self::new().0
	}
}
