use {
	super::{
		EditorApp,
		action::AppAction,
		state::{RESIZE_REFLOW_INTERVAL, ResizeCoalescer},
	},
	crate::{
		editor::{EditorEditIntent, EditorIntent, EditorModeIntent, EditorMotion, EditorPointerIntent},
		types::{ControlsMessage, Message, SidebarTab, ViewportMessage},
	},
	iced::{Size, Vector},
	std::time::Instant,
};

fn assert_approx_eq(left: f32, right: f32) {
	assert!((left - right).abs() <= 0.001, "left={left} right={right}");
}

fn metric_samples(app: &EditorApp, label: &str) -> u64 {
	app.store.perf.metric_total_samples(label)
}

fn editor(intent: EditorIntent) -> AppAction {
	AppAction::editor(intent)
}

fn resize(size: Size) -> AppAction {
	AppAction::ObserveCanvasResize(size)
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
	app.store.dispatch(resize(Size::new(760.0, 280.0)));
	app.store.dispatch(AppAction::FlushResizeReflow);

	for _ in 0..5 {
		app.store.dispatch(editor(EditorIntent::Motion(EditorMotion::Down)));
	}

	let target = app
		.store
		.session
		.view_state()
		.viewport_target
		.expect("selection should expose a viewport target");
	app.store.state.viewport.canvas_scroll = Vector::new(0.0, (target.y - 40.0).max(0.0));
	let previous_scroll = app.store.state.viewport.canvas_scroll;

	app.store
		.dispatch(editor(EditorIntent::Mode(EditorModeIntent::EnterInsertAfter)));
	app.store
		.dispatch(editor(EditorIntent::Edit(EditorEditIntent::InsertText(
			"!".to_string(),
		))));

	assert_eq!(app.store.state.viewport.canvas_scroll, previous_scroll);
}

#[test]
fn keyboard_motion_reveals_caret_when_it_leaves_viewport() {
	let (mut app, _) = EditorApp::new();
	app.store.dispatch(resize(Size::new(760.0, 220.0)));

	for _ in 0..12 {
		app.store.dispatch(editor(EditorIntent::Motion(EditorMotion::Down)));
	}

	assert!(app.store.state.viewport.canvas_scroll.y > 0.0);
}

#[test]
fn keyboard_motion_keeps_the_selected_preset() {
	let (mut app, _) = EditorApp::new();

	app.store.dispatch(editor(EditorIntent::Motion(EditorMotion::Right)));

	assert_eq!(app.store.state.controls.preset, crate::types::SamplePreset::Tall);
}

#[test]
fn text_edits_flip_the_preset_to_custom() {
	let (mut app, _) = EditorApp::new();

	app.store
		.dispatch(editor(EditorIntent::Mode(EditorModeIntent::EnterInsertAfter)));
	app.store
		.dispatch(editor(EditorIntent::Edit(EditorEditIntent::InsertText(
			"!".to_string(),
		))));

	assert_eq!(app.store.state.controls.preset, crate::types::SamplePreset::Custom);
}

#[test]
fn controls_tab_keeps_text_edits_on_the_hot_path() {
	let (mut app, _) = EditorApp::new();
	let revision_before = app.store.session.snapshot().scene.as_ref().map(|scene| scene.revision);
	let scene_builds_before = app.store.session.derived_scene_build_count();

	app.store
		.dispatch(editor(EditorIntent::Mode(EditorModeIntent::EnterInsertAfter)));
	app.store
		.dispatch(editor(EditorIntent::Edit(EditorEditIntent::InsertText(
			"!".to_string(),
		))));

	assert_eq!(
		app.store.session.snapshot().scene.as_ref().map(|scene| scene.revision),
		revision_before
	);
	assert_eq!(app.store.session.derived_scene_build_count(), scene_builds_before);
	assert!(app.store.session.snapshot().scene.is_none());
	assert!(app.store.session.text().contains('!'));
	assert_eq!(
		app.store.session.snapshot().editor.editor_bytes,
		app.store.session.text().len()
	);
}

#[test]
fn resize_reflow_updates_scene_when_inspect_is_active() {
	let (mut app, _) = EditorApp::new();
	app.store.dispatch(AppAction::SelectSidebarTab(SidebarTab::Inspect));
	let revision_before = app
		.store
		.session
		.snapshot()
		.scene
		.as_ref()
		.map_or(0, |scene| scene.revision);

	app.store.dispatch(resize(Size::new(980.0, 280.0)));
	app.store.dispatch(AppAction::FlushResizeReflow);

	assert!(
		app.store
			.session
			.snapshot()
			.scene
			.as_ref()
			.map_or(0, |scene| scene.revision)
			> revision_before
	);
	assert_approx_eq(
		app.store
			.session
			.snapshot()
			.scene
			.as_ref()
			.expect("inspect mode should materialize a derived scene")
			.layout
			.max_width,
		app.store.state.viewport.layout_width,
	);
}

#[test]
fn resize_bursts_only_sync_editor_width_on_coalesced_widths() {
	let (mut app, _) = EditorApp::new();

	app.store.dispatch(resize(Size::new(980.0, 280.0)));
	assert_eq!(metric_samples(&app, "editor.width_sync"), 0);

	app.store.dispatch(resize(Size::new(920.0, 280.0)));
	app.store.dispatch(resize(Size::new(860.0, 280.0)));
	assert_eq!(metric_samples(&app, "editor.width_sync"), 0);

	app.store.dispatch(AppAction::FlushResizeReflow);
	assert_eq!(metric_samples(&app, "editor.width_sync"), 1);
}

#[test]
fn leaving_inspect_clears_hover_and_selection() {
	let (mut app, _) = EditorApp::new();

	app.store.dispatch(AppAction::SelectSidebarTab(SidebarTab::Inspect));
	app.store
		.dispatch(AppAction::HoverCanvas(Some(crate::types::CanvasTarget::Run(0))));
	app.store.dispatch(AppAction::BeginPointerSelection {
		target: Some(crate::types::CanvasTarget::Run(0)),
		intent: EditorPointerIntent::Begin {
			position: iced::Point::ORIGIN,
			select_word: false,
		},
	});
	assert!(app.store.state.sidebar.hovered_target.is_some());
	assert!(app.store.state.sidebar.selected_target.is_some());

	app.store.dispatch(AppAction::SelectSidebarTab(SidebarTab::Controls));

	assert_eq!(app.store.state.sidebar.hovered_target, None);
	assert_eq!(app.store.state.sidebar.selected_target, None);
}

#[test]
fn canvas_generated_editor_intents_flow_through_session() {
	let (mut app, _) = EditorApp::new();

	app.store.dispatch(AppAction::BeginPointerSelection {
		target: Some(crate::types::CanvasTarget::Run(0)),
		intent: EditorPointerIntent::Begin {
			position: iced::Point::new(30.0, 32.0),
			select_word: false,
		},
	});
	app.store
		.dispatch(editor(EditorIntent::Pointer(EditorPointerIntent::Drag(
			iced::Point::new(120.0, 32.0),
		))));

	assert!(
		app.store
			.session
			.view_state()
			.selection
			.is_some_and(|selection| selection.end > selection.start)
	);
}

#[test]
fn inspect_sidebar_cache_reuses_model_until_inputs_change() {
	let (mut app, _) = EditorApp::new();
	app.store.dispatch(AppAction::SelectSidebarTab(SidebarTab::Inspect));

	app.test_view_sidebar();
	app.test_view_sidebar();
	assert_eq!(app.store.sidebar_cache.inspect_build_count(), 1);

	app.store
		.dispatch(AppAction::HoverCanvas(Some(crate::types::CanvasTarget::Run(0))));
	app.test_view_sidebar();
	assert_eq!(app.store.sidebar_cache.inspect_build_count(), 2);
}

#[test]
fn perf_sidebar_cache_reuses_model_until_metrics_change() {
	let (mut app, _) = EditorApp::new();
	app.store.dispatch(AppAction::SelectSidebarTab(SidebarTab::Perf));

	app.test_view_sidebar();
	app.test_view_sidebar();
	assert_eq!(app.store.sidebar_cache.perf_build_count(), 1);

	app.store
		.dispatch(editor(EditorIntent::Mode(EditorModeIntent::EnterInsertAfter)));
	app.test_view_sidebar();
	assert_eq!(app.store.sidebar_cache.perf_build_count(), 2);
}

#[test]
fn repeated_no_op_inputs_do_not_churn_scene_state() {
	let (mut app, _) = EditorApp::new();
	let revision_before = app.store.session.snapshot().scene.as_ref().map(|scene| scene.revision);

	app.store.dispatch(AppAction::Control(ControlsMessage::FontSelected(
		app.store.state.controls.font,
	)));
	app.store
		.dispatch(AppAction::Control(ControlsMessage::ShowHitboxesChanged(
			app.store.state.controls.show_hitboxes,
		)));

	app.store
		.dispatch(editor(EditorIntent::Mode(EditorModeIntent::EnterInsertAfter)));
	app.store
		.dispatch(editor(EditorIntent::Edit(EditorEditIntent::InsertText(
			"!".to_string(),
		))));

	assert_eq!(
		app.store.session.snapshot().scene.as_ref().map(|scene| scene.revision),
		revision_before
	);
	assert!(app.store.session.snapshot().scene.is_none());
}

#[test]
fn width_sync_keeps_the_selected_preset() {
	let (mut app, _) = EditorApp::new();

	app.store.dispatch(resize(Size::new(980.0, 280.0)));
	app.store.dispatch(AppAction::FlushResizeReflow);

	assert_eq!(app.store.state.controls.preset, crate::types::SamplePreset::Tall);
}

#[test]
fn config_changes_keep_the_selected_preset() {
	let (mut app, _) = EditorApp::new();

	app.store.dispatch(AppAction::Control(ControlsMessage::FontSelected(
		crate::types::FontChoice::Monospace,
	)));
	app.store.dispatch(AppAction::Control(ControlsMessage::WrappingSelected(
		crate::types::WrapChoice::Glyph,
	)));

	assert_eq!(app.store.state.controls.preset, crate::types::SamplePreset::Tall);
}

#[test]
fn update_maps_editor_messages_to_commands() {
	let (mut app, _) = EditorApp::new();

	std::mem::drop(app.update(Message::Editor(EditorIntent::Motion(EditorMotion::Right))));

	assert!(
		app.store
			.session
			.view_state()
			.selection
			.is_some_and(|selection| selection.end > selection.start)
	);
}

#[test]
fn update_maps_viewport_messages_to_commands() {
	let (mut app, _) = EditorApp::new();

	std::mem::drop(app.update(Message::Viewport(ViewportMessage::CanvasResized(Size::new(
		980.0, 280.0,
	)))));
	std::mem::drop(app.update(Message::Viewport(ViewportMessage::ResizeTick(
		Instant::now() + RESIZE_REFLOW_INTERVAL,
	))));

	assert_eq!(metric_samples(&app, "editor.width_sync"), 1);
}
