use {
	super::{
		geometry::{scroll_delta, to_scene_local},
		state::{CanvasIntent, DecodedEvent},
	},
	crate::{
		editor::{EditorEditIntent, EditorHistoryIntent, EditorIntent, EditorMode, EditorModeIntent, EditorMotion},
		scene::LayoutScene,
	},
	iced::{
		Rectangle,
		keyboard::{self, key},
		mouse,
		widget::canvas,
		window,
	},
	std::time::Instant,
};

pub(super) fn decode_event(
	mode: EditorMode, focused: bool, event: &canvas::Event, scene: &LayoutScene, bounds: Rectangle,
	cursor: mouse::Cursor, scroll: iced::Vector,
) -> Option<DecodedEvent> {
	let cursor_position = cursor.position_in(bounds);
	let cursor_local = cursor_position.map(|position| to_scene_local(position, scroll));
	let cursor_target = cursor_local.and_then(|position| scene.hit_test(position));

	match event {
		canvas::Event::Mouse(mouse::Event::WheelScrolled { delta }) if cursor.is_over(bounds) => {
			Some(DecodedEvent::canvas(CanvasIntent::WheelScrolled(scroll_delta(*delta))))
		}
		canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => Some(DecodedEvent::canvas(cursor_local.map_or(
			CanvasIntent::CursorLeft,
			|position| CanvasIntent::CursorMoved {
				position,
				target: cursor_target,
			},
		))),
		canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => Some(DecodedEvent::canvas(
			cursor_local.map_or(CanvasIntent::Blur, |position| CanvasIntent::PointerPressed {
				position,
				target: cursor_target,
				at: Instant::now(),
			}),
		)),
		canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
			Some(DecodedEvent::canvas(CanvasIntent::PointerReleased))
		}
		canvas::Event::Keyboard(keyboard::Event::KeyPressed {
			key,
			physical_key,
			modifiers,
			text,
			..
		}) if focused => key_intent(mode, key, *physical_key, *modifiers, text.as_deref())
			.map(|intent| DecodedEvent::new(CanvasIntent::RetainFocus, Some(intent))),
		canvas::Event::Window(window::Event::RedrawRequested(_)) => {
			Some(DecodedEvent::canvas(CanvasIntent::RedrawRequested))
		}
		_ => None,
	}
}

fn key_intent(
	mode: EditorMode, key: &keyboard::Key, physical_key: key::Physical, modifiers: keyboard::Modifiers,
	text: Option<&str>,
) -> Option<EditorIntent> {
	let latin = key
		.to_latin(physical_key)
		.map(|character| character.to_ascii_lowercase());
	let redo_modifier = modifiers.shift();

	if modifiers.command() {
		return match latin {
			Some('z') if redo_modifier => Some(EditorIntent::History(EditorHistoryIntent::Redo)),
			Some('z') => Some(EditorIntent::History(EditorHistoryIntent::Undo)),
			Some('y') => Some(EditorIntent::History(EditorHistoryIntent::Redo)),
			_ => None,
		};
	}

	match mode {
		EditorMode::Normal => {
			if modifiers.alt() {
				return None;
			}

			match key.as_ref() {
				key::Key::Named(key::Named::ArrowLeft) => Some(EditorIntent::Motion(EditorMotion::Left)),
				key::Key::Named(key::Named::ArrowRight) => Some(EditorIntent::Motion(EditorMotion::Right)),
				key::Key::Named(key::Named::ArrowUp) => Some(EditorIntent::Motion(EditorMotion::Up)),
				key::Key::Named(key::Named::ArrowDown) => Some(EditorIntent::Motion(EditorMotion::Down)),
				key::Key::Named(key::Named::Home) => Some(EditorIntent::Motion(EditorMotion::LineStart)),
				key::Key::Named(key::Named::End) => Some(EditorIntent::Motion(EditorMotion::LineEnd)),
				key::Key::Named(key::Named::Backspace | key::Named::Delete) => {
					Some(EditorIntent::Edit(EditorEditIntent::DeleteSelection))
				}
				key::Key::Named(key::Named::Enter) => Some(EditorIntent::Mode(EditorModeIntent::EnterInsertAfter)),
				key::Key::Named(key::Named::Escape) => Some(EditorIntent::Mode(EditorModeIntent::ExitInsert)),
				_ => match latin {
					Some('h') => Some(EditorIntent::Motion(EditorMotion::Left)),
					Some('l') => Some(EditorIntent::Motion(EditorMotion::Right)),
					Some('k') => Some(EditorIntent::Motion(EditorMotion::Up)),
					Some('j') => Some(EditorIntent::Motion(EditorMotion::Down)),
					Some('i') => Some(EditorIntent::Mode(EditorModeIntent::EnterInsertBefore)),
					Some('a') => Some(EditorIntent::Mode(EditorModeIntent::EnterInsertAfter)),
					Some('x') => Some(EditorIntent::Edit(EditorEditIntent::DeleteSelection)),
					_ => None,
				},
			}
		}
		EditorMode::Insert => match key.as_ref() {
			key::Key::Named(key::Named::ArrowLeft) => Some(EditorIntent::Motion(EditorMotion::Left)),
			key::Key::Named(key::Named::ArrowRight) => Some(EditorIntent::Motion(EditorMotion::Right)),
			key::Key::Named(key::Named::ArrowUp) => Some(EditorIntent::Motion(EditorMotion::Up)),
			key::Key::Named(key::Named::ArrowDown) => Some(EditorIntent::Motion(EditorMotion::Down)),
			key::Key::Named(key::Named::Home) => Some(EditorIntent::Motion(EditorMotion::LineStart)),
			key::Key::Named(key::Named::End) => Some(EditorIntent::Motion(EditorMotion::LineEnd)),
			key::Key::Named(key::Named::Backspace) => Some(EditorIntent::Edit(EditorEditIntent::Backspace)),
			key::Key::Named(key::Named::Delete) => Some(EditorIntent::Edit(EditorEditIntent::DeleteForward)),
			key::Key::Named(key::Named::Enter) => {
				Some(EditorIntent::Edit(EditorEditIntent::InsertText("\n".to_string())))
			}
			key::Key::Named(key::Named::Tab) => {
				Some(EditorIntent::Edit(EditorEditIntent::InsertText("\t".to_string())))
			}
			key::Key::Named(key::Named::Escape) => Some(EditorIntent::Mode(EditorModeIntent::ExitInsert)),
			_ => {
				if modifiers.alt() {
					return None;
				}

				text.filter(|text| !text.chars().all(char::is_control))
					.map(|text| EditorIntent::Edit(EditorEditIntent::InsertText(text.to_string())))
			}
		},
	}
}
