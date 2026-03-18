use {
	super::{
		EditorApp,
		headless::{
			headless_delete_seed_char_count, headless_incremental_line_break_steps, headless_incremental_typing_steps,
			headless_large_paste_chunk_len, headless_undo_redo_steps,
		},
	},
	crate::{HeadlessScriptScenario, types::SidebarTab},
};

#[test]
fn large_paste_script_scenario_applies_one_large_edit() {
	let mut app = EditorApp::headless();
	app.headless_driver()
		.configure_script_scenario(HeadlessScriptScenario::LargePaste);
	let before = app.store.session.text().len();
	let history_before = app.store.session.history_depths();

	app.headless_driver()
		.run_script_scenario(HeadlessScriptScenario::LargePaste);

	assert_eq!(
		app.store.session.text().len(),
		before + headless_large_paste_chunk_len()
	);
	assert_eq!(app.store.session.history_depths().0 - history_before.0, 1);
	assert_eq!(app.store.session.history_depths().1, 0);
}

#[test]
fn incremental_typing_script_scenario_records_many_small_edits() {
	let mut app = EditorApp::headless();
	app.headless_driver()
		.configure_script_scenario(HeadlessScriptScenario::IncrementalTyping);
	let before = app.store.session.text().len();
	let history_before = app.store.session.history_depths();

	app.headless_driver()
		.run_script_scenario(HeadlessScriptScenario::IncrementalTyping);

	assert_eq!(
		app.store.session.text().len(),
		before + headless_incremental_typing_steps()
	);
	assert_eq!(
		app.store.session.history_depths().0 - history_before.0,
		headless_incremental_typing_steps()
	);
}

#[test]
fn incremental_line_break_script_scenario_grows_line_count() {
	let mut app = EditorApp::headless();
	app.headless_driver()
		.configure_script_scenario(HeadlessScriptScenario::IncrementalLineBreaks);
	let before_lines = app.store.session.text().lines().count();
	let history_before = app.store.session.history_depths();

	app.headless_driver()
		.run_script_scenario(HeadlessScriptScenario::IncrementalLineBreaks);

	assert_eq!(
		app.store.session.history_depths().0 - history_before.0,
		headless_incremental_line_break_steps()
	);
	assert!(app.store.session.text().lines().count() >= before_lines + headless_incremental_line_break_steps());
	assert!(app.store.session.text().contains("branch 0047"));
}

#[test]
fn undo_redo_script_scenario_reaches_the_redone_state() {
	let mut app = EditorApp::headless();
	app.headless_driver()
		.configure_script_scenario(HeadlessScriptScenario::UndoRedoBurst);
	let before = app.store.session.text().len();

	app.headless_driver()
		.run_script_scenario(HeadlessScriptScenario::UndoRedoBurst);

	assert!(app.store.session.text().len() > before);
	assert_eq!(app.store.session.history_depths(), (headless_undo_redo_steps(), 0));
	assert!(app.store.session.text().contains("u47"));
}

#[test]
fn backspace_script_scenario_removes_the_seeded_insert() {
	let mut app = EditorApp::headless();
	app.headless_driver()
		.configure_script_scenario(HeadlessScriptScenario::BackspaceBurst);
	let before = app.store.session.text().len();

	app.headless_driver()
		.run_script_scenario(HeadlessScriptScenario::BackspaceBurst);

	assert_eq!(
		app.store.session.text().len() + headless_delete_seed_char_count(),
		before
	);
	assert_eq!(app.store.session.history_depths(), (256, 0));
}

#[test]
fn delete_forward_script_scenario_removes_the_seeded_insert() {
	let mut app = EditorApp::headless();
	app.headless_driver()
		.configure_script_scenario(HeadlessScriptScenario::DeleteForwardBurst);
	let before = app.store.session.text().len();

	app.headless_driver()
		.run_script_scenario(HeadlessScriptScenario::DeleteForwardBurst);

	assert_eq!(
		app.store.session.text().len() + headless_delete_seed_char_count(),
		before
	);
	assert_eq!(app.store.session.history_depths(), (256, 0));
}

#[test]
fn motion_sweep_script_scenario_moves_without_editing() {
	let mut app = EditorApp::headless();
	app.headless_driver()
		.configure_script_scenario(HeadlessScriptScenario::MotionSweep);
	let before = app.store.session.text().to_string();
	let history_before = app.store.session.history_depths();

	app.headless_driver()
		.run_script_scenario(HeadlessScriptScenario::MotionSweep);

	assert_eq!(app.store.session.text(), before);
	assert_eq!(app.store.session.history_depths(), history_before);
	assert!(app.store.session.view_state().selection.is_some());
}

#[test]
fn pointer_selection_sweep_script_scenario_expands_selection() {
	let mut app = EditorApp::headless();
	app.headless_driver()
		.configure_script_scenario(HeadlessScriptScenario::PointerSelectionSweep);

	app.headless_driver()
		.run_script_scenario(HeadlessScriptScenario::PointerSelectionSweep);

	assert!(
		app.store
			.session
			.view_state()
			.selection
			.is_some_and(|selection| selection.end > selection.start)
	);
}

#[test]
fn resize_reflow_script_scenario_changes_layout_width_and_revisions() {
	let mut app = EditorApp::headless();
	app.headless_driver()
		.configure_script_scenario(HeadlessScriptScenario::ResizeReflowSweep);
	let revision_before = app.store.session.snapshot().scene.as_ref().map(|scene| scene.revision);

	app.headless_driver()
		.run_script_scenario(HeadlessScriptScenario::ResizeReflowSweep);

	assert_eq!(
		app.store.session.snapshot().scene.as_ref().map(|scene| scene.revision),
		revision_before
	);
	assert!(app.store.perf.metric_total_samples("editor.width_sync") > 0);
	assert!(app.store.session.snapshot().scene.is_none());
}

#[test]
fn perf_incremental_typing_step_mutates_editor_state() {
	let mut app = EditorApp::headless();
	app.headless_driver()
		.configure_perf_scenario(crate::PerfScenario::IncrementalTyping);
	let before = app.store.session.text().len();
	let history_before = app.store.session.history_depths().0;

	app.headless_driver()
		.run_perf_step(crate::PerfScenario::IncrementalTyping, 0);

	assert_eq!(app.store.session.text().len(), before + 1);
	assert_eq!(app.store.session.history_depths().0, history_before + 1);
	assert!(app.store.session.snapshot().scene.is_none());
}

#[test]
fn perf_resize_reflow_step_rebuilds_immediately_when_scene_ui_is_active() {
	let mut app = EditorApp::headless();
	app.headless_driver()
		.configure_perf_scenario(crate::PerfScenario::ResizeReflow);
	let revision_before = app
		.store
		.session
		.snapshot()
		.scene
		.as_ref()
		.map_or(0, |scene| scene.revision);

	app.headless_driver()
		.run_perf_step(crate::PerfScenario::ResizeReflow, 0);

	assert!(
		app.store
			.session
			.snapshot()
			.scene
			.as_ref()
			.map_or(0, |scene| scene.revision)
			> revision_before
	);
}

#[test]
fn inspect_interaction_script_scenario_keeps_inspect_state_active() {
	let mut app = EditorApp::headless();
	app.headless_driver()
		.configure_script_scenario(HeadlessScriptScenario::InspectInteractionSweep);

	app.headless_driver()
		.run_script_scenario(HeadlessScriptScenario::InspectInteractionSweep);

	assert_eq!(app.store.state.sidebar.active_tab, SidebarTab::Inspect);
	assert!(app.store.state.controls.show_hitboxes);
	assert!(app.store.state.controls.show_baselines);
	assert!(app.store.state.sidebar.selected_target.is_some());
}
