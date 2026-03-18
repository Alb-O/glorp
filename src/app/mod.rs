mod action;
mod headless;
#[cfg(test)]
mod headless_tests;
mod presenter;
mod reducer;
mod session;
mod sidebar_cache;
mod sidebar_data;
mod state;
mod store;
#[cfg(test)]
mod tests;
mod view;

use {
	self::{action::AppAction, store::AppStore},
	crate::types::Message,
	iced::{Element, Subscription, Task},
};

pub struct EditorApp {
	store: AppStore,
}

impl EditorApp {
	pub(crate) fn new() -> (Self, Task<Message>) {
		(Self { store: AppStore::new() }, Task::none())
	}

	#[must_use]
	pub(crate) fn headless() -> Self {
		Self::new().0
	}

	pub(crate) fn subscription(&self) -> Subscription<Message> {
		self.store.subscription()
	}

	pub(crate) fn update(&mut self, message: Message) -> Task<Message> {
		self.store.dispatch(AppAction::from(message));
		self.store.perf.flush_canvas_metrics();
		Task::none()
	}

	pub(crate) fn view(&self) -> Element<'_, Message> {
		self.store.view()
	}

	pub(crate) fn headless_view(&self) -> Element<'_, ()> {
		self.view().map(|_| ())
	}

	#[cfg(test)]
	pub(super) fn test_view_sidebar(&self) -> Element<'_, Message> {
		self.store.view_sidebar_for_test()
	}
}
