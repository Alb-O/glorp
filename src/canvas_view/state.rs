use std::cell::Cell;
use std::time::Instant;

use iced::widget::canvas;
use iced::{Point, Vector};

use crate::editor::{EditorIntent, EditorPointerIntent};
use crate::types::{CanvasEvent, CanvasTarget, Message};

use super::geometry::{
	DOUBLE_CLICK_DISTANCE, DOUBLE_CLICK_INTERVAL, animate_scroll, clamp_scroll, point_distance, vector_length,
};

#[derive(Debug, Default)]
pub(crate) struct CanvasState {
	hovered_target: Option<CanvasTarget>,
	focused: bool,
	scroll: Vector,
	target_scroll: Vector,
	pointer_selecting: bool,
	last_click: Option<(Instant, Point)>,
	scene_cache: canvas::Cache,
	cached_scene_revision: Cell<Option<u64>>,
	cached_scroll: Cell<Option<(i32, i32)>>,
}

#[derive(Debug, Clone)]
pub(super) struct DecodedEvent {
	pub(super) canvas_intent: CanvasIntent,
	pub(super) editor_intent: Option<EditorIntent>,
}

impl DecodedEvent {
	pub(super) fn new(canvas_intent: CanvasIntent, editor_intent: Option<EditorIntent>) -> Self {
		Self {
			canvas_intent,
			editor_intent,
		}
	}

	pub(super) fn canvas(canvas_intent: CanvasIntent) -> Self {
		Self::new(canvas_intent, None)
	}
}

#[derive(Debug, Clone)]
pub(super) enum CanvasIntent {
	WheelScrolled(Vector),
	CursorMoved {
		position: Point,
		target: Option<CanvasTarget>,
	},
	PointerPressed {
		position: Point,
		target: Option<CanvasTarget>,
		at: Instant,
	},
	PointerReleased,
	RedrawRequested,
	CursorLeft,
	Blur,
	RetainFocus,
}

#[derive(Debug)]
pub(super) enum CanvasAction {
	None,
	RequestRedraw(bool),
	Publish(Message, bool),
}

impl CanvasAction {
	pub(super) fn publish_canvas(event: CanvasEvent, capture: bool) -> Self {
		Self::Publish(Message::Canvas(event), capture)
	}

	pub(super) fn publish_editor(intent: EditorIntent, capture: bool) -> Self {
		Self::Publish(Message::Editor(intent), capture)
	}

	pub(super) fn into_iced(self) -> Option<canvas::Action<Message>> {
		match self {
			Self::None => None,
			Self::RequestRedraw(capture) => {
				let action = canvas::Action::request_redraw();
				Some(if capture { action.and_capture() } else { action })
			}
			Self::Publish(message, capture) => {
				let action = canvas::Action::publish(message);
				Some(if capture { action.and_capture() } else { action })
			}
		}
	}
}

impl CanvasState {
	pub(super) fn focused(&self) -> bool {
		self.focused
	}

	pub(super) fn scroll(&self) -> Vector {
		self.scroll
	}

	pub(super) fn clamp_to_bounds(&mut self, max_scroll: Vector) {
		self.target_scroll = clamp_scroll(self.target_scroll, max_scroll);
		self.scroll = clamp_scroll(self.scroll, max_scroll);
	}

	pub(super) fn transition(&mut self, event: DecodedEvent, max_scroll: Vector) -> CanvasAction {
		self.clamp_to_bounds(max_scroll);

		if let Some(intent) = event.editor_intent {
			return CanvasAction::publish_editor(intent, true);
		}

		match event.canvas_intent {
			CanvasIntent::WheelScrolled(delta) => {
				self.focused = true;
				self.target_scroll = clamp_scroll(self.target_scroll + delta, max_scroll);

				if vector_length(self.target_scroll - self.scroll) > 0.1 {
					CanvasAction::RequestRedraw(true)
				} else {
					CanvasAction::None
				}
			}
			CanvasIntent::CursorMoved { position, target } => {
				if self.pointer_selecting {
					CanvasAction::publish_editor(
						EditorIntent::Pointer(EditorPointerIntent::DragSelection(position)),
						true,
					)
				} else if self.hovered_target != target {
					self.hovered_target = target;
					CanvasAction::publish_canvas(CanvasEvent::Hovered(target), false)
				} else {
					CanvasAction::None
				}
			}
			CanvasIntent::PointerPressed { position, target, at } => {
				self.focused = true;
				self.pointer_selecting = true;
				self.hovered_target = target;
				let double_click = self.last_click.is_some_and(|(last_at, last_position)| {
					at.duration_since(last_at) <= DOUBLE_CLICK_INTERVAL
						&& point_distance(last_position, position) <= DOUBLE_CLICK_DISTANCE
				});
				self.last_click = Some((at, position));

				CanvasAction::publish_canvas(
					CanvasEvent::PointerSelectionStarted {
						target,
						intent: EditorPointerIntent::BeginSelection {
							position,
							select_word: double_click,
						},
					},
					true,
				)
			}
			CanvasIntent::PointerReleased => {
				if self.pointer_selecting {
					self.pointer_selecting = false;
					CanvasAction::publish_editor(EditorIntent::Pointer(EditorPointerIntent::EndSelection), false)
				} else {
					CanvasAction::None
				}
			}
			CanvasIntent::RedrawRequested => {
				let previous_scroll = self.scroll;
				let next_scroll = animate_scroll(self.scroll, self.target_scroll);
				if vector_length(next_scroll - self.scroll) > 0.01 {
					self.scroll = clamp_scroll(next_scroll, max_scroll);
					CanvasAction::publish_canvas(CanvasEvent::ScrollChanged(self.scroll), false)
				} else {
					self.scroll = clamp_scroll(self.target_scroll, max_scroll);
					if vector_length(self.scroll - previous_scroll) > 0.01 {
						CanvasAction::publish_canvas(CanvasEvent::ScrollChanged(self.scroll), false)
					} else {
						CanvasAction::None
					}
				}
			}
			CanvasIntent::CursorLeft => {
				if self.hovered_target.take().is_some() {
					CanvasAction::publish_canvas(CanvasEvent::Hovered(None), false)
				} else {
					CanvasAction::None
				}
			}
			CanvasIntent::Blur => {
				self.focused = false;
				CanvasAction::None
			}
			CanvasIntent::RetainFocus => CanvasAction::None,
		}
	}

	pub(super) fn scene_cache(&self) -> &canvas::Cache {
		&self.scene_cache
	}

	pub(super) fn cache_miss(&self, scene_revision: u64, scroll: Vector) -> bool {
		let cached_scroll = (scroll.x.round() as i32, scroll.y.round() as i32);
		self.cached_scene_revision.get() != Some(scene_revision) || self.cached_scroll.get() != Some(cached_scroll)
	}

	pub(super) fn refresh_cache_key(&self, scene_revision: u64, scroll: Vector) {
		self.scene_cache.clear();
		self.cached_scene_revision.set(Some(scene_revision));
		self.cached_scroll
			.set(Some((scroll.x.round() as i32, scroll.y.round() as i32)));
	}
}

#[cfg(test)]
mod tests {
	use super::{CanvasAction, CanvasIntent, CanvasState, DecodedEvent};
	use crate::editor::{EditorIntent, EditorPointerIntent};
	use crate::types::{CanvasEvent, CanvasTarget, Message};
	use iced::{Point, Vector};
	use std::time::{Duration, Instant};

	fn max_scroll() -> Vector {
		Vector::new(600.0, 900.0)
	}

	#[test]
	fn double_click_detection_flips_word_selection_flag() {
		let mut state = CanvasState::default();
		let position = Point::new(12.0, 18.0);
		let started = Instant::now();

		let first = state.transition(
			DecodedEvent::canvas(CanvasIntent::PointerPressed {
				position,
				target: Some(CanvasTarget::Run(1)),
				at: started,
			}),
			max_scroll(),
		);
		let second = state.transition(
			DecodedEvent::canvas(CanvasIntent::PointerPressed {
				position,
				target: Some(CanvasTarget::Run(1)),
				at: started + Duration::from_millis(120),
			}),
			max_scroll(),
		);

		let CanvasAction::Publish(Message::Canvas(CanvasEvent::PointerSelectionStarted { intent, .. }), _) = first
		else {
			panic!("expected pointer selection start");
		};
		assert_eq!(
			intent,
			EditorPointerIntent::BeginSelection {
				position,
				select_word: false,
			}
		);

		let CanvasAction::Publish(Message::Canvas(CanvasEvent::PointerSelectionStarted { intent, .. }), _) = second
		else {
			panic!("expected pointer selection start");
		};
		assert_eq!(
			intent,
			EditorPointerIntent::BeginSelection {
				position,
				select_word: true,
			}
		);
	}

	#[test]
	fn focus_transitions_follow_pointer_press_and_blur() {
		let mut state = CanvasState::default();

		assert!(!state.focused());

		let _ = state.transition(
			DecodedEvent::canvas(CanvasIntent::PointerPressed {
				position: Point::new(4.0, 4.0),
				target: None,
				at: Instant::now(),
			}),
			max_scroll(),
		);
		assert!(state.focused());

		let _ = state.transition(DecodedEvent::canvas(CanvasIntent::Blur), max_scroll());
		assert!(!state.focused());
	}

	#[test]
	fn wheel_scroll_updates_animation_target() {
		let mut state = CanvasState::default();

		let action = state.transition(
			DecodedEvent::canvas(CanvasIntent::WheelScrolled(Vector::new(0.0, 120.0))),
			max_scroll(),
		);

		assert!(matches!(action, CanvasAction::RequestRedraw(true)));
		assert!(state.target_scroll.y > 0.0);
	}

	#[test]
	fn hover_clears_when_cursor_leaves_bounds() {
		let mut state = CanvasState::default();
		let _ = state.transition(
			DecodedEvent::canvas(CanvasIntent::CursorMoved {
				position: Point::new(5.0, 5.0),
				target: Some(CanvasTarget::Run(0)),
			}),
			max_scroll(),
		);

		let action = state.transition(DecodedEvent::canvas(CanvasIntent::CursorLeft), max_scroll());

		assert!(matches!(
			action,
			CanvasAction::Publish(Message::Canvas(CanvasEvent::Hovered(None)), false)
		));
	}

	#[test]
	fn cache_invalidates_on_scene_revision_or_scroll_change() {
		let state = CanvasState::default();

		assert!(state.cache_miss(1, Vector::ZERO));
		state.refresh_cache_key(1, Vector::ZERO);
		assert!(!state.cache_miss(1, Vector::ZERO));
		assert!(state.cache_miss(2, Vector::ZERO));
		assert!(state.cache_miss(1, Vector::new(10.0, 0.0)));
	}

	#[test]
	fn dragging_pointer_selection_publishes_editor_pointer_intent() {
		let mut state = CanvasState::default();
		let _ = state.transition(
			DecodedEvent::canvas(CanvasIntent::PointerPressed {
				position: Point::new(1.0, 1.0),
				target: None,
				at: Instant::now(),
			}),
			max_scroll(),
		);

		let action = state.transition(
			DecodedEvent::canvas(CanvasIntent::CursorMoved {
				position: Point::new(24.0, 36.0),
				target: Some(CanvasTarget::Run(0)),
			}),
			max_scroll(),
		);

		assert!(matches!(
			action,
			CanvasAction::Publish(
				Message::Editor(EditorIntent::Pointer(EditorPointerIntent::DragSelection(_))),
				true
			)
		));
	}
}
