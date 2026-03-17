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
	iced::{Element, Subscription, Task},
};

pub(super) struct AppModel {
	session: DocumentSession,
	controls: ControlsState,
	sidebar: SidebarState,
	viewport: ViewportState,
	shell: ShellState,
	perf: PerfMonitor,
	sidebar_cache: SidebarCache,
}

pub struct EditorApp {
	model: AppModel,
}

impl AppModel {
	fn new() -> Self {
		let controls = ControlsState::new();
		let viewport = ViewportState::new(ControlsState::initial_layout_width());
		let session = DocumentSession::new(controls.preset.text(), controls.scene_config(viewport.layout_width));

		Self {
			session,
			controls,
			sidebar: SidebarState::new(),
			viewport,
			shell: ShellState::new(),
			perf: PerfMonitor::default(),
			sidebar_cache: SidebarCache::default(),
		}
	}

	fn scene_config(&self) -> SceneConfig {
		self.controls.scene_config(self.viewport.layout_width)
	}
}

impl EditorApp {
	pub(crate) fn new() -> (Self, Task<Message>) {
		(Self { model: AppModel::new() }, Task::none())
	}

	#[must_use]
	pub(crate) fn headless() -> Self {
		Self::new().0
	}

	pub(crate) fn subscription(&self) -> Subscription<Message> {
		self.model.subscription()
	}

	pub(crate) fn update(&mut self, message: Message) -> Task<Message> {
		self.model.update(message)
	}

	pub(crate) fn view(&self) -> Element<'_, Message> {
		self.model.view()
	}

	pub(crate) fn headless_view(&self) -> Element<'_, ()> {
		self.model.headless_view()
	}

	#[cfg(test)]
	pub(super) fn test_view_sidebar(&self) -> Element<'_, Message> {
		self.model.test_view_sidebar()
	}
}
