use {
	super::geometry::{
		DOUBLE_CLICK_DISTANCE, DOUBLE_CLICK_INTERVAL, animate_scroll, clamp_scroll, point_distance, vector_length,
	},
	crate::{
		editor::{EditorIntent, EditorPointerIntent},
		types::{CanvasEvent, CanvasTarget, Message},
	},
	iced::{Point, Vector, widget::canvas},
	std::time::Instant,
};

const SCROLL_SETTLE_EPSILON: f32 = 0.01;

#[derive(Debug, Default)]
pub(crate) struct CanvasState {
	hovered_target: Option<CanvasTarget>,
	focused: bool,
	scroll: Vector,
	target_scroll: Vector,
	pointer_selecting: bool,
	last_click: Option<(Instant, Point)>,
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
		let (action, capture) = match self {
			Self::None => return None,
			Self::RequestRedraw(capture) => (canvas::Action::request_redraw(), capture),
			Self::Publish(message, capture) => (canvas::Action::publish(message), capture),
		};
		Some(if capture { action.and_capture() } else { action })
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
				let focus_changed = !self.focused;
				self.focused = true;
				self.target_scroll = clamp_scroll(self.target_scroll + delta, max_scroll);

				if vector_length(self.target_scroll - self.scroll) > 0.1 {
					CanvasAction::RequestRedraw(true)
				} else if focus_changed {
					CanvasAction::publish_canvas(CanvasEvent::ScrollChanged(self.scroll), false)
				} else {
					CanvasAction::None
				}
			}
			CanvasIntent::CursorMoved { position, target } => {
				if self.pointer_selecting {
					CanvasAction::publish_editor(EditorIntent::Pointer(EditorPointerIntent::Drag(position)), true)
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
						intent: EditorPointerIntent::Begin {
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
					CanvasAction::publish_editor(EditorIntent::Pointer(EditorPointerIntent::End), false)
				} else {
					CanvasAction::None
				}
			}
			CanvasIntent::RedrawRequested => {
				let animated_scroll = animate_scroll(self.scroll, self.target_scroll);
				let next_scroll = if vector_length(animated_scroll - self.scroll) > SCROLL_SETTLE_EPSILON {
					clamp_scroll(animated_scroll, max_scroll)
				} else {
					clamp_scroll(self.target_scroll, max_scroll)
				};

				self.publish_scroll_if_changed(next_scroll)
			}
			CanvasIntent::CursorLeft => {
				if self.hovered_target.take().is_some() {
					CanvasAction::publish_canvas(CanvasEvent::Hovered(None), false)
				} else {
					CanvasAction::None
				}
			}
			CanvasIntent::Blur => {
				if self.focused {
					self.focused = false;
					CanvasAction::publish_canvas(CanvasEvent::FocusChanged(false), false)
				} else {
					CanvasAction::None
				}
			}
			CanvasIntent::RetainFocus => CanvasAction::None,
		}
	}

	fn publish_scroll_if_changed(&mut self, next_scroll: Vector) -> CanvasAction {
		let previous_scroll = self.scroll;
		self.scroll = next_scroll;

		// Match redraw settling: once movement is below epsilon, stop publishing
		// "changed" events for the final float noise.
		if vector_length(self.scroll - previous_scroll) > SCROLL_SETTLE_EPSILON {
			CanvasAction::publish_canvas(CanvasEvent::ScrollChanged(self.scroll), false)
		} else {
			CanvasAction::None
		}
	}
}

#[cfg(test)]
mod tests {
	use {
		super::{CanvasAction, CanvasIntent, CanvasState, DecodedEvent},
		crate::{
			editor::{EditorIntent, EditorPointerIntent},
			types::{CanvasEvent, CanvasTarget, Message},
		},
		iced::{Point, Vector},
		std::time::{Duration, Instant},
	};

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
			EditorPointerIntent::Begin {
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
			EditorPointerIntent::Begin {
				position,
				select_word: true,
			}
		);
	}

	#[test]
	fn focus_transitions_follow_pointer_press_and_blur() {
		let mut state = CanvasState::default();

		assert!(!state.focused());

		state.transition(
			DecodedEvent::canvas(CanvasIntent::PointerPressed {
				position: Point::new(4.0, 4.0),
				target: None,
				at: Instant::now(),
			}),
			max_scroll(),
		);
		assert!(state.focused());

		state.transition(DecodedEvent::canvas(CanvasIntent::Blur), max_scroll());
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
		state.transition(
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
	fn dragging_pointer_selection_publishes_editor_pointer_intent() {
		let mut state = CanvasState::default();
		state.transition(
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
				Message::Editor(EditorIntent::Pointer(EditorPointerIntent::Drag(_))),
				true
			)
		));
	}
}
