use std::time::{Duration, Instant};

use iced::{Size, Vector};

use super::Playground;
use super::state::{RESIZE_REFLOW_INTERVAL, ResizeCoalescer};
use crate::editor::{EditorEditIntent, EditorIntent, EditorModeIntent, EditorMotion, EditorPointerIntent};
use crate::types::{CanvasEvent, Message, SidebarMessage, SidebarTab, ViewportMessage};

fn editor(intent: EditorIntent) -> Message {
	Message::Editor(intent)
}

fn resize(size: Size) -> Message {
	Message::Viewport(ViewportMessage::CanvasResized(size))
}

#[test]
fn resize_coalescer_limits_burst_reflows_and_flushes_latest_width() {
	let started = Instant::now();
	let mut coalescer = ResizeCoalescer::new(600.0);

	assert_eq!(coalescer.observe(700.0, started), Some(700.0));
	assert_eq!(coalescer.observe(710.0, started + Duration::from_millis(4)), None);
	assert_eq!(coalescer.observe(720.0, started + Duration::from_millis(8)), None);
	assert!(coalescer.has_pending());
	assert_eq!(coalescer.flush(started + RESIZE_REFLOW_INTERVAL), Some(720.0));
	assert!(!coalescer.has_pending());
}

#[test]
fn edits_preserve_visible_scroll_position() {
	let (mut playground, _) = Playground::new();
	let _ = playground.update(resize(Size::new(760.0, 280.0)));

	for _ in 0..5 {
		let _ = playground.update(editor(EditorIntent::Motion(EditorMotion::Down)));
	}

	let target = playground
		.session
		.view_state()
		.viewport_target
		.expect("selection should expose a viewport target");
	playground.viewport.canvas_scroll = Vector::new(0.0, (target.y - 40.0).max(0.0));
	let previous_scroll = playground.viewport.canvas_scroll;

	let _ = playground.update(editor(EditorIntent::Mode(EditorModeIntent::EnterInsertAfter)));
	let _ = playground.update(editor(EditorIntent::Edit(EditorEditIntent::InsertText(
		"!".to_string(),
	))));

	assert_eq!(playground.viewport.canvas_scroll, previous_scroll);
}

#[test]
fn keyboard_motion_reveals_caret_when_it_leaves_viewport() {
	let (mut playground, _) = Playground::new();
	let _ = playground.update(resize(Size::new(760.0, 220.0)));

	for _ in 0..12 {
		let _ = playground.update(editor(EditorIntent::Motion(EditorMotion::Down)));
	}

	assert!(playground.viewport.canvas_scroll.y > 0.0);
}

#[test]
fn keyboard_motion_keeps_the_selected_preset() {
	let (mut playground, _) = Playground::new();

	let _ = playground.update(editor(EditorIntent::Motion(EditorMotion::Right)));

	assert_eq!(playground.controls.preset, crate::types::SamplePreset::Tall);
}

#[test]
fn text_edits_flip_the_preset_to_custom() {
	let (mut playground, _) = Playground::new();

	let _ = playground.update(editor(EditorIntent::Mode(EditorModeIntent::EnterInsertAfter)));
	let _ = playground.update(editor(EditorIntent::Edit(EditorEditIntent::InsertText(
		"!".to_string(),
	))));

	assert_eq!(playground.controls.preset, crate::types::SamplePreset::Custom);
}

#[test]
fn leaving_inspect_clears_hover_and_selection() {
	let (mut playground, _) = Playground::new();

	let _ = playground.update(Message::Sidebar(SidebarMessage::SelectTab(SidebarTab::Inspect)));
	let _ = playground.update(Message::Canvas(CanvasEvent::Hovered(Some(
		crate::types::CanvasTarget::Run(0),
	))));
	let _ = playground.update(Message::Canvas(CanvasEvent::PointerSelectionStarted {
		target: Some(crate::types::CanvasTarget::Run(0)),
		intent: EditorPointerIntent::BeginSelection {
			position: iced::Point::ORIGIN,
			select_word: false,
		},
	}));
	assert!(playground.sidebar.hovered_target.is_some());
	assert!(playground.sidebar.selected_target.is_some());

	let _ = playground.update(Message::Sidebar(SidebarMessage::SelectTab(SidebarTab::Controls)));

	assert_eq!(playground.sidebar.hovered_target, None);
	assert_eq!(playground.sidebar.selected_target, None);
}

#[test]
fn canvas_generated_editor_intents_flow_through_session() {
	let (mut playground, _) = Playground::new();

	let _ = playground.update(Message::Canvas(CanvasEvent::PointerSelectionStarted {
		target: Some(crate::types::CanvasTarget::Run(0)),
		intent: EditorPointerIntent::BeginSelection {
			position: iced::Point::new(30.0, 32.0),
			select_word: false,
		},
	}));
	let _ = playground.update(editor(EditorIntent::Pointer(EditorPointerIntent::DragSelection(
		iced::Point::new(120.0, 32.0),
	))));

	assert!(
		playground
			.session
			.view_state()
			.selection
			.is_some_and(|selection| selection.end > selection.start)
	);
}
