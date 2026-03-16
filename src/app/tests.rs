use {
	super::{
		Playground,
		state::{RESIZE_REFLOW_INTERVAL, ResizeCoalescer},
	},
	crate::{
		editor::{EditorEditIntent, EditorIntent, EditorModeIntent, EditorMotion, EditorPointerIntent},
		types::{CanvasEvent, Message, SidebarMessage, SidebarTab, ViewportMessage},
	},
	iced::{Size, Vector},
	std::time::{Duration, Instant},
};

fn assert_approx_eq(left: f32, right: f32) {
	assert!((left - right).abs() <= 0.001, "left={left} right={right}");
}

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
fn controls_tab_defers_scene_rebuild_until_inspect_needs_it() {
	let (mut playground, _) = Playground::new();
	let revision_before = playground.viewport.scene_revision;
	let scene_text_before = playground.session.scene().text.to_string();

	let _ = playground.update(editor(EditorIntent::Mode(EditorModeIntent::EnterInsertAfter)));
	let _ = playground.update(editor(EditorIntent::Edit(EditorEditIntent::InsertText(
		"!".to_string(),
	))));

	assert!(playground.scene_dirty);
	assert_eq!(playground.viewport.scene_revision, revision_before);
	assert_eq!(playground.session.text().len(), scene_text_before.len() + 1);
	assert!(playground.session.text().contains('!'));
	assert_eq!(playground.session.scene().text.as_ref(), scene_text_before);

	let _ = playground.update(Message::Sidebar(SidebarMessage::SelectTab(SidebarTab::Inspect)));

	assert!(!playground.scene_dirty);
	assert!(playground.viewport.scene_revision > revision_before);
	assert_eq!(playground.session.scene().text.as_ref(), playground.session.text());
}

#[test]
fn controls_tab_defers_resize_reflow_until_scene_ui_needs_it() {
	let (mut playground, _) = Playground::new();
	let revision_before = playground.viewport.scene_revision;
	let scene_width_before = playground.session.scene().max_width;

	let _ = playground.update(resize(Size::new(980.0, 280.0)));
	let _ = playground.update(Message::Viewport(ViewportMessage::ResizeTick(
		Instant::now() + RESIZE_REFLOW_INTERVAL,
	)));

	assert!(playground.scene_dirty);
	assert!(playground.deferred_resize_reflow);
	assert_eq!(playground.viewport.scene_revision, revision_before);
	assert!((playground.viewport.layout_width - scene_width_before).abs() > 0.001);
	assert_approx_eq(playground.session.scene().max_width, scene_width_before);

	let _ = playground.update(Message::Sidebar(SidebarMessage::SelectTab(SidebarTab::Inspect)));

	assert!(!playground.scene_dirty);
	assert!(!playground.deferred_resize_reflow);
	assert!(playground.viewport.scene_revision > revision_before);
	assert_approx_eq(playground.session.scene().max_width, playground.viewport.layout_width);
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
		intent: EditorPointerIntent::Begin {
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
		intent: EditorPointerIntent::Begin {
			position: iced::Point::new(30.0, 32.0),
			select_word: false,
		},
	}));
	let _ = playground.update(editor(EditorIntent::Pointer(EditorPointerIntent::Drag(
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
