use {
	super::{
		Playground,
		headless::{
			headless_delete_seed_char_count, headless_incremental_line_break_steps, headless_incremental_typing_steps,
			headless_large_paste_chunk_len, headless_undo_redo_steps,
		},
	},
	crate::{HeadlessScriptScenario, types::SidebarTab},
};

#[test]
fn large_paste_script_scenario_applies_one_large_edit() {
	let mut playground = Playground::headless();
	playground.configure_headless_script_scenario(HeadlessScriptScenario::LargePaste);
	let before = playground.session.text().len();
	let history_before = playground.session.history_depths();

	let _ = playground.run_headless_script_scenario(HeadlessScriptScenario::LargePaste);

	assert_eq!(
		playground.session.text().len(),
		before + headless_large_paste_chunk_len()
	);
	assert_eq!(playground.session.history_depths().0 - history_before.0, 1);
	assert_eq!(playground.session.history_depths().1, 0);
}

#[test]
fn incremental_typing_script_scenario_records_many_small_edits() {
	let mut playground = Playground::headless();
	playground.configure_headless_script_scenario(HeadlessScriptScenario::IncrementalTyping);
	let before = playground.session.text().len();
	let history_before = playground.session.history_depths();

	let _ = playground.run_headless_script_scenario(HeadlessScriptScenario::IncrementalTyping);

	assert_eq!(
		playground.session.text().len(),
		before + headless_incremental_typing_steps()
	);
	assert_eq!(
		playground.session.history_depths().0 - history_before.0,
		headless_incremental_typing_steps()
	);
}

#[test]
fn incremental_line_break_script_scenario_grows_line_count() {
	let mut playground = Playground::headless();
	playground.configure_headless_script_scenario(HeadlessScriptScenario::IncrementalLineBreaks);
	let before_lines = playground.session.text().lines().count();
	let history_before = playground.session.history_depths();

	let _ = playground.run_headless_script_scenario(HeadlessScriptScenario::IncrementalLineBreaks);

	assert_eq!(
		playground.session.history_depths().0 - history_before.0,
		headless_incremental_line_break_steps()
	);
	assert!(playground.session.text().lines().count() >= before_lines + headless_incremental_line_break_steps());
	assert!(playground.session.text().contains("branch 0047"));
}

#[test]
fn undo_redo_script_scenario_reaches_the_redone_state() {
	let mut playground = Playground::headless();
	playground.configure_headless_script_scenario(HeadlessScriptScenario::UndoRedoBurst);
	let before = playground.session.text().len();

	let _ = playground.run_headless_script_scenario(HeadlessScriptScenario::UndoRedoBurst);

	assert!(playground.session.text().len() > before);
	assert_eq!(playground.session.history_depths(), (headless_undo_redo_steps(), 0));
	assert!(playground.session.text().contains("u47"));
}

#[test]
fn backspace_script_scenario_removes_the_seeded_insert() {
	let mut playground = Playground::headless();
	playground.configure_headless_script_scenario(HeadlessScriptScenario::BackspaceBurst);
	let before = playground.session.text().len();

	let _ = playground.run_headless_script_scenario(HeadlessScriptScenario::BackspaceBurst);

	assert_eq!(
		playground.session.text().len() + headless_delete_seed_char_count(),
		before
	);
	assert_eq!(playground.session.history_depths(), (256, 0));
}

#[test]
fn delete_forward_script_scenario_removes_the_seeded_insert() {
	let mut playground = Playground::headless();
	playground.configure_headless_script_scenario(HeadlessScriptScenario::DeleteForwardBurst);
	let before = playground.session.text().len();

	let _ = playground.run_headless_script_scenario(HeadlessScriptScenario::DeleteForwardBurst);

	assert_eq!(
		playground.session.text().len() + headless_delete_seed_char_count(),
		before
	);
	assert_eq!(playground.session.history_depths(), (256, 0));
}

#[test]
fn motion_sweep_script_scenario_moves_without_editing() {
	let mut playground = Playground::headless();
	playground.configure_headless_script_scenario(HeadlessScriptScenario::MotionSweep);
	let before = playground.session.text().to_string();
	let history_before = playground.session.history_depths();

	let _ = playground.run_headless_script_scenario(HeadlessScriptScenario::MotionSweep);

	assert_eq!(playground.session.text(), before);
	assert_eq!(playground.session.history_depths(), history_before);
	assert!(playground.session.view_state().selection.is_some());
}

#[test]
fn pointer_selection_sweep_script_scenario_expands_selection() {
	let mut playground = Playground::headless();
	playground.configure_headless_script_scenario(HeadlessScriptScenario::PointerSelectionSweep);

	let _ = playground.run_headless_script_scenario(HeadlessScriptScenario::PointerSelectionSweep);

	assert!(
		playground
			.session
			.view_state()
			.selection
			.is_some_and(|selection| selection.end > selection.start)
	);
}

#[test]
fn resize_reflow_script_scenario_changes_layout_width_and_revisions() {
	let mut playground = Playground::headless();
	playground.configure_headless_script_scenario(HeadlessScriptScenario::ResizeReflowSweep);
	let width_before = playground.viewport.layout_width;
	let revision_before = playground.viewport.scene_revision;

	let _ = playground.run_headless_script_scenario(HeadlessScriptScenario::ResizeReflowSweep);

	assert_ne!(playground.viewport.layout_width, width_before);
	assert_eq!(playground.viewport.scene_revision, revision_before);
	assert!(playground.scene_dirty);
	assert!(playground.deferred_resize_reflow);

	let _ = playground.update(crate::types::Message::Sidebar(crate::types::SidebarMessage::SelectTab(
		crate::types::SidebarTab::Inspect,
	)));

	assert!(playground.viewport.scene_revision > revision_before);
	assert!(!playground.scene_dirty);
	assert!(!playground.deferred_resize_reflow);
}

#[test]
fn inspect_interaction_script_scenario_keeps_inspect_state_active() {
	let mut playground = Playground::headless();
	playground.configure_headless_script_scenario(HeadlessScriptScenario::InspectInteractionSweep);

	let _ = playground.run_headless_script_scenario(HeadlessScriptScenario::InspectInteractionSweep);

	assert_eq!(playground.sidebar.active_tab, SidebarTab::Inspect);
	assert!(playground.controls.show_hitboxes);
	assert!(playground.controls.show_baselines);
	assert!(playground.sidebar.selected_target.is_some());
}
