use {
	super::{
		EditorApp,
		state::{RESIZE_REFLOW_INTERVAL, ResizeCoalescer},
	},
	crate::{
		editor::{EditorEditIntent, EditorIntent, EditorModeIntent, EditorMotion, EditorPointerIntent},
		types::{CanvasEvent, ControlsMessage, Message, SidebarMessage, SidebarTab, ViewportMessage},
	},
	iced::{Size, Vector},
	std::time::Instant,
};

fn assert_approx_eq(left: f32, right: f32) {
	assert!((left - right).abs() <= 0.001, "left={left} right={right}");
}

fn metric_samples(app: &EditorApp, label: &str) -> u64 {
	app.perf.metric_total_samples(label)
}

fn editor(intent: EditorIntent) -> Message {
	Message::Editor(intent)
}

fn resize(size: Size) -> Message {
	Message::Viewport(ViewportMessage::CanvasResized(size))
}

#[test]
fn resize_coalescer_limits_burst_reflows_and_flushes_latest_width() {
	let mut coalescer = ResizeCoalescer::new(600.0);

	coalescer.observe(700.0);
	coalescer.observe(710.0);
	coalescer.observe(720.0);
	assert!(coalescer.has_pending());
	assert_eq!(coalescer.flush(), Some(720.0));
	assert!(!coalescer.has_pending());
}

#[test]
fn edits_preserve_visible_scroll_position() {
	let (mut app, _) = EditorApp::new();
	let _ = app.update(resize(Size::new(760.0, 280.0)));
	let _ = app.update(Message::Viewport(ViewportMessage::ResizeTick(
		Instant::now() + RESIZE_REFLOW_INTERVAL,
	)));

	for _ in 0..5 {
		let _ = app.update(editor(EditorIntent::Motion(EditorMotion::Down)));
	}

	let target = app
		.session
		.view_state()
		.viewport_target
		.expect("selection should expose a viewport target");
	app.viewport.canvas_scroll = Vector::new(0.0, (target.y - 40.0).max(0.0));
	let previous_scroll = app.viewport.canvas_scroll;

	let _ = app.update(editor(EditorIntent::Mode(EditorModeIntent::EnterInsertAfter)));
	let _ = app.update(editor(EditorIntent::Edit(EditorEditIntent::InsertText(
		"!".to_string(),
	))));

	assert_eq!(app.viewport.canvas_scroll, previous_scroll);
}

#[test]
fn keyboard_motion_reveals_caret_when_it_leaves_viewport() {
	let (mut app, _) = EditorApp::new();
	let _ = app.update(resize(Size::new(760.0, 220.0)));

	for _ in 0..12 {
		let _ = app.update(editor(EditorIntent::Motion(EditorMotion::Down)));
	}

	assert!(app.viewport.canvas_scroll.y > 0.0);
}

#[test]
fn keyboard_motion_keeps_the_selected_preset() {
	let (mut app, _) = EditorApp::new();

	let _ = app.update(editor(EditorIntent::Motion(EditorMotion::Right)));

	assert_eq!(app.controls.preset, crate::types::SamplePreset::Tall);
}

#[test]
fn text_edits_flip_the_preset_to_custom() {
	let (mut app, _) = EditorApp::new();

	let _ = app.update(editor(EditorIntent::Mode(EditorModeIntent::EnterInsertAfter)));
	let _ = app.update(editor(EditorIntent::Edit(EditorEditIntent::InsertText(
		"!".to_string(),
	))));

	assert_eq!(app.controls.preset, crate::types::SamplePreset::Custom);
}

#[test]
fn controls_tab_keeps_text_edits_on_the_hot_path() {
	let (mut app, _) = EditorApp::new();
	let revision_before = app.session.snapshot().scene.as_ref().map(|scene| scene.revision);
	let scene_builds_before = app.session.derived_scene_build_count();

	let _ = app.update(editor(EditorIntent::Mode(EditorModeIntent::EnterInsertAfter)));
	let _ = app.update(editor(EditorIntent::Edit(EditorEditIntent::InsertText(
		"!".to_string(),
	))));

	assert_eq!(
		app.session.snapshot().scene.as_ref().map(|scene| scene.revision),
		revision_before
	);
	assert_eq!(app.session.derived_scene_build_count(), scene_builds_before);
	assert!(app.session.snapshot().scene.is_none());
	assert!(app.session.text().contains('!'));
	assert_eq!(app.session.snapshot().editor.editor_bytes(), app.session.text().len());
}

#[test]
fn resize_reflow_updates_scene_when_inspect_is_active() {
	let (mut app, _) = EditorApp::new();
	let _ = app.update(Message::Sidebar(SidebarMessage::SelectTab(SidebarTab::Inspect)));
	let revision_before = app.session.snapshot().scene.as_ref().map_or(0, |scene| scene.revision);

	let _ = app.update(resize(Size::new(980.0, 280.0)));
	let _ = app.update(Message::Viewport(ViewportMessage::ResizeTick(
		Instant::now() + RESIZE_REFLOW_INTERVAL,
	)));

	assert!(app.session.snapshot().scene.as_ref().map_or(0, |scene| scene.revision) > revision_before);
	assert_approx_eq(
		app.session
			.snapshot()
			.scene
			.as_ref()
			.expect("inspect mode should materialize a derived scene")
			.layout
			.max_width,
		app.viewport.layout_width,
	);
}

#[test]
fn resize_bursts_only_sync_editor_width_on_coalesced_widths() {
	let (mut app, _) = EditorApp::new();

	let _ = app.update(resize(Size::new(980.0, 280.0)));
	assert_eq!(metric_samples(&app, "editor.width_sync"), 0);

	let _ = app.update(resize(Size::new(920.0, 280.0)));
	let _ = app.update(resize(Size::new(860.0, 280.0)));
	assert_eq!(metric_samples(&app, "editor.width_sync"), 0);

	let _ = app.update(Message::Viewport(ViewportMessage::ResizeTick(
		Instant::now() + RESIZE_REFLOW_INTERVAL,
	)));
	assert_eq!(metric_samples(&app, "editor.width_sync"), 1);
}

#[test]
fn leaving_inspect_clears_hover_and_selection() {
	let (mut app, _) = EditorApp::new();

	let _ = app.update(Message::Sidebar(SidebarMessage::SelectTab(SidebarTab::Inspect)));
	let _ = app.update(Message::Canvas(CanvasEvent::Hovered(Some(
		crate::types::CanvasTarget::Run(0),
	))));
	let _ = app.update(Message::Canvas(CanvasEvent::PointerSelectionStarted {
		target: Some(crate::types::CanvasTarget::Run(0)),
		intent: EditorPointerIntent::Begin {
			position: iced::Point::ORIGIN,
			select_word: false,
		},
	}));
	assert!(app.sidebar.hovered_target.is_some());
	assert!(app.sidebar.selected_target.is_some());

	let _ = app.update(Message::Sidebar(SidebarMessage::SelectTab(SidebarTab::Controls)));

	assert_eq!(app.sidebar.hovered_target, None);
	assert_eq!(app.sidebar.selected_target, None);
}

#[test]
fn canvas_generated_editor_intents_flow_through_session() {
	let (mut app, _) = EditorApp::new();

	let _ = app.update(Message::Canvas(CanvasEvent::PointerSelectionStarted {
		target: Some(crate::types::CanvasTarget::Run(0)),
		intent: EditorPointerIntent::Begin {
			position: iced::Point::new(30.0, 32.0),
			select_word: false,
		},
	}));
	let _ = app.update(editor(EditorIntent::Pointer(EditorPointerIntent::Drag(
		iced::Point::new(120.0, 32.0),
	))));

	assert!(
		app.session
			.view_state()
			.selection
			.is_some_and(|selection| selection.end > selection.start)
	);
}

#[test]
fn inspect_sidebar_cache_reuses_model_until_inputs_change() {
	let (mut app, _) = EditorApp::new();
	let _ = app.update(Message::Sidebar(SidebarMessage::SelectTab(SidebarTab::Inspect)));

	let _ = app.test_view_sidebar();
	let _ = app.test_view_sidebar();
	assert_eq!(app.sidebar_cache.inspect_build_count(), 1);

	let _ = app.update(Message::Canvas(CanvasEvent::Hovered(Some(
		crate::types::CanvasTarget::Run(0),
	))));
	let _ = app.test_view_sidebar();
	assert_eq!(app.sidebar_cache.inspect_build_count(), 2);
}

#[test]
fn perf_sidebar_cache_reuses_model_until_metrics_change() {
	let (mut app, _) = EditorApp::new();
	let _ = app.update(Message::Sidebar(SidebarMessage::SelectTab(SidebarTab::Perf)));

	let _ = app.test_view_sidebar();
	let _ = app.test_view_sidebar();
	assert_eq!(app.sidebar_cache.perf_build_count(), 1);

	let _ = app.update(editor(EditorIntent::Mode(EditorModeIntent::EnterInsertAfter)));
	let _ = app.test_view_sidebar();
	assert_eq!(app.sidebar_cache.perf_build_count(), 2);
}

#[test]
fn repeated_no_op_inputs_do_not_churn_scene_state() {
	let (mut app, _) = EditorApp::new();
	let revision_before = app.session.snapshot().scene.as_ref().map(|scene| scene.revision);

	let _ = app.update(Message::Controls(ControlsMessage::FontSelected(app.controls.font)));
	let _ = app.update(Message::Controls(ControlsMessage::ShowHitboxesChanged(
		app.controls.show_hitboxes,
	)));

	let _ = app.update(editor(EditorIntent::Mode(EditorModeIntent::EnterInsertAfter)));
	let _ = app.update(editor(EditorIntent::Edit(EditorEditIntent::InsertText(
		"!".to_string(),
	))));

	assert_eq!(
		app.session.snapshot().scene.as_ref().map(|scene| scene.revision),
		revision_before
	);
	assert!(app.session.snapshot().scene.is_none());
}
