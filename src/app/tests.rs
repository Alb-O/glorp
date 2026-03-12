use std::time::{Duration, Instant};

use iced::{Size, Vector};

use super::Playground;
use super::state::{RESIZE_REFLOW_INTERVAL, ResizeCoalescer};
use crate::editor::EditorCommand;
use crate::types::{Message, SidebarTab};

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
	let _ = playground.update(Message::CanvasViewportResized(Size::new(760.0, 280.0)));

	for _ in 0..5 {
		let _ = playground.update(Message::EditorCommand(EditorCommand::MoveDown));
	}

	let target = playground
		.session
		.view_state()
		.viewport_target
		.expect("selection should expose a viewport target");
	playground.viewport.canvas_scroll = Vector::new(0.0, (target.y - 40.0).max(0.0));
	let previous_scroll = playground.viewport.canvas_scroll;

	let _ = playground.update(Message::EditorCommand(EditorCommand::EnterInsertAfter));
	let _ = playground.update(Message::EditorCommand(EditorCommand::InsertText("!".to_string())));

	assert_eq!(playground.viewport.canvas_scroll, previous_scroll);
}

#[test]
fn keyboard_motion_reveals_caret_when_it_leaves_viewport() {
	let (mut playground, _) = Playground::new();
	let _ = playground.update(Message::CanvasViewportResized(Size::new(760.0, 220.0)));

	for _ in 0..12 {
		let _ = playground.update(Message::EditorCommand(EditorCommand::MoveDown));
	}

	assert!(playground.viewport.canvas_scroll.y > 0.0);
}

#[test]
fn keyboard_motion_keeps_the_selected_preset() {
	let (mut playground, _) = Playground::new();

	let _ = playground.update(Message::EditorCommand(EditorCommand::MoveRight));

	assert_eq!(playground.controls.preset, crate::types::SamplePreset::Tall);
}

#[test]
fn text_edits_flip_the_preset_to_custom() {
	let (mut playground, _) = Playground::new();

	let _ = playground.update(Message::EditorCommand(EditorCommand::EnterInsertAfter));
	let _ = playground.update(Message::EditorCommand(EditorCommand::InsertText("!".to_string())));

	assert_eq!(playground.controls.preset, crate::types::SamplePreset::Custom);
}

#[test]
fn leaving_inspect_clears_hover_and_selection() {
	let (mut playground, _) = Playground::new();

	let _ = playground.update(Message::SelectSidebarTab(SidebarTab::Inspect));
	let _ = playground.update(Message::CanvasHovered(Some(crate::types::CanvasTarget::Run(0))));
	let _ = playground.update(Message::CanvasPressed {
		target: Some(crate::types::CanvasTarget::Run(0)),
		position: iced::Point::ORIGIN,
		double_click: false,
	});
	assert!(playground.sidebar.hovered_target.is_some());
	assert!(playground.sidebar.selected_target.is_some());

	let _ = playground.update(Message::SelectSidebarTab(SidebarTab::Controls));

	assert_eq!(playground.sidebar.hovered_target, None);
	assert_eq!(playground.sidebar.selected_target, None);
}
