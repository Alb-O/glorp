mod session;
mod state;
#[cfg(test)]
mod tests;
mod update;
mod view;

use iced::Task;

use crate::HeadlessScenario;
use crate::perf::PerfMonitor;
use crate::scene::SceneConfig;
use crate::types::{CanvasEvent, ControlsMessage, Message, SamplePreset, SidebarMessage, SidebarTab, ViewportMessage};

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

	pub fn configure_headless_scenario(&mut self, scenario: HeadlessScenario) {
		let _ = self.update(Message::Viewport(ViewportMessage::CanvasResized(iced::Size::new(
			1600.0, 1000.0,
		))));

		match scenario {
			HeadlessScenario::Default => {}
			HeadlessScenario::Tall => {
				let _ = self.update(Message::Controls(ControlsMessage::LoadPreset(SamplePreset::Tall)));
			}
			HeadlessScenario::TallInspect => {
				let _ = self.update(Message::Controls(ControlsMessage::LoadPreset(SamplePreset::Tall)));
				let _ = self.update(Message::Controls(ControlsMessage::ShowHitboxesChanged(true)));
				let _ = self.update(Message::Controls(ControlsMessage::ShowBaselinesChanged(true)));
				let _ = self.update(Message::Sidebar(SidebarMessage::SelectTab(SidebarTab::Inspect)));
				let _ = self.update(Message::Canvas(CanvasEvent::Hovered(Some(
					crate::types::CanvasTarget::Glyph {
						run_index: 0,
						glyph_index: 0,
					},
				))));
			}
			HeadlessScenario::TallPerf => {
				let _ = self.update(Message::Controls(ControlsMessage::LoadPreset(SamplePreset::Tall)));
				let _ = self.update(Message::Sidebar(SidebarMessage::SelectTab(SidebarTab::Perf)));
			}
		}
	}
}
