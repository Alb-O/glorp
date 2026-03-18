use {
	crate::{
		runtime::GlorpRuntime,
		state::{SessionDelta, SessionRequest},
	},
	glorp_api::{
		ConfigAssignment, ConfigPath, EditorHistoryCommand, EditorModeCommand, EditorMotion, GlorpConfig, GlorpDelta,
		GlorpError, GlorpExec, GlorpOutcome, GlorpRevisions, GlorpTxn,
	},
	glorp_editor::{
		EditorEditIntent, EditorHistoryIntent, EditorIntent, EditorModeIntent, EditorMotion as EngineMotion,
		EditorPointerIntent,
	},
};

pub fn execute(runtime: &mut GlorpRuntime, exec: GlorpExec) -> Result<GlorpOutcome, GlorpError> {
	match exec {
		GlorpExec::Txn(txn) => execute_txn(runtime, txn),
		GlorpExec::ConfigSet(input) => execute_config_set(runtime, input),
		GlorpExec::ConfigReset(input) => execute_config_reset(runtime, input),
		GlorpExec::ConfigPatch(input) => execute_config_patch(runtime, input),
		GlorpExec::ConfigReload => execute_config_reload(runtime),
		GlorpExec::ConfigPersist => execute_config_persist(runtime),
		GlorpExec::DocumentReplace(input) => Ok(publish_session(runtime, SessionRequest::ReplaceDocument(input.text))),
		GlorpExec::EditorMotion(input) => Ok(execute_editor_motion(runtime, input.motion)),
		GlorpExec::EditorMode(input) => Ok(execute_editor_mode(runtime, input.mode)),
		GlorpExec::EditorInsert(input) => Ok(execute_editor_edit(runtime, EditorEditIntent::InsertText(input.text))),
		GlorpExec::EditorBackspace => Ok(execute_editor_edit(runtime, EditorEditIntent::Backspace)),
		GlorpExec::EditorDeleteForward => Ok(execute_editor_edit(runtime, EditorEditIntent::DeleteForward)),
		GlorpExec::EditorDeleteSelection => Ok(execute_editor_edit(runtime, EditorEditIntent::DeleteSelection)),
		GlorpExec::EditorHistory(input) => Ok(execute_editor_history(runtime, input.action)),
		GlorpExec::EditorPointerBegin(input) => Ok(execute_editor_pointer(
			runtime,
			EditorPointerIntent::Begin {
				position: iced::Point::new(input.x, input.y),
				select_word: input.select_word,
			},
		)),
		GlorpExec::EditorPointerDrag(input) => Ok(execute_editor_pointer(
			runtime,
			EditorPointerIntent::Drag(iced::Point::new(input.x, input.y)),
		)),
		GlorpExec::EditorPointerEnd => Ok(execute_editor_pointer(runtime, EditorPointerIntent::End)),
		GlorpExec::UiSidebarSelect(input) => Ok(execute_ui(runtime, |state| state.active_tab = input.tab)),
		GlorpExec::UiInspectTargetHover(input) => Ok(execute_ui(runtime, |state| state.hovered_target = input.target)),
		GlorpExec::UiInspectTargetSelect(input) => {
			Ok(execute_ui(runtime, |state| state.selected_target = input.target))
		}
		GlorpExec::UiCanvasFocusSet(input) => Ok(execute_ui(runtime, |state| state.canvas_focused = input.focused)),
		GlorpExec::UiViewportScrollTo(input) => Ok(execute_ui(runtime, |state| {
			state.canvas_scroll_x = input.x.max(0.0);
			state.canvas_scroll_y = input.y.max(0.0);
		})),
		GlorpExec::UiViewportMetricsSet(input) => Ok(execute_viewport_metrics(runtime, input)),
		GlorpExec::UiPaneRatioSet(input) => Ok(execute_ui(runtime, |state| {
			state.pane_ratio = input.ratio.clamp(0.1, 0.9)
		})),
		GlorpExec::SceneEnsure => Ok(execute_scene_ensure(runtime)),
	}
}

fn execute_txn(runtime: &mut GlorpRuntime, txn: GlorpTxn) -> Result<GlorpOutcome, GlorpError> {
	let checkpoint = runtime.state.checkpoint();
	let previous_events = runtime.subscriptions_state();

	txn.execs
		.into_iter()
		.try_fold(GlorpOutcome::default(), |mut accumulated, exec| {
			if matches!(exec, GlorpExec::Txn(_)) {
				return Err(GlorpError::validation(None, "nested transactions are not supported"));
			}
			merge_outcome(&mut accumulated, execute(runtime, exec)?);
			Ok(accumulated)
		})
		.inspect_err(|_| {
			runtime.state.restore(checkpoint);
			runtime.restore_subscriptions(previous_events);
		})
}

fn execute_config_set(runtime: &mut GlorpRuntime, input: ConfigAssignment) -> Result<GlorpOutcome, GlorpError> {
	runtime.state.config.set_path(&input.path, &input.value)?;
	publish_config(runtime, vec![input.path])
}

fn execute_config_reset(
	runtime: &mut GlorpRuntime, input: glorp_api::ConfigPathInput,
) -> Result<GlorpOutcome, GlorpError> {
	runtime.state.config.reset_path(&input.path)?;
	publish_config(runtime, vec![input.path])
}

fn execute_config_patch(
	runtime: &mut GlorpRuntime, input: glorp_api::ConfigPatchInput,
) -> Result<GlorpOutcome, GlorpError> {
	let assignments = flatten_patch(&input.patch)?;
	let changed_paths = runtime.state.config.patch(&assignments)?;
	publish_config(runtime, changed_paths)
}

fn execute_config_reload(runtime: &mut GlorpRuntime) -> Result<GlorpOutcome, GlorpError> {
	runtime.state.config = runtime.config_store.load()?;
	let changed_paths = GlorpConfig::schema_defaults()
		.into_iter()
		.map(|(path, _)| path)
		.collect();
	publish_config(runtime, changed_paths)
}

fn execute_config_persist(runtime: &mut GlorpRuntime) -> Result<GlorpOutcome, GlorpError> {
	runtime.config_store.save(&runtime.state.config)?;
	Ok(GlorpOutcome {
		revisions: runtime.state.revisions,
		..GlorpOutcome::default()
	})
}

fn publish_config(runtime: &mut GlorpRuntime, changed_paths: Vec<ConfigPath>) -> Result<GlorpOutcome, GlorpError> {
	let mut outcome = run_session(runtime, SessionRequest::SyncConfig);
	runtime.state.revisions.config += 1;
	outcome.revisions = runtime.state.revisions;
	outcome.delta.config_changed = !changed_paths.is_empty();
	outcome.changed_config_paths = changed_paths;
	runtime.publish_changed(&outcome);
	Ok(outcome)
}

fn execute_editor_motion(runtime: &mut GlorpRuntime, motion: EditorMotion) -> GlorpOutcome {
	let motion = match motion {
		EditorMotion::Left => EngineMotion::Left,
		EditorMotion::Right => EngineMotion::Right,
		EditorMotion::Up => EngineMotion::Up,
		EditorMotion::Down => EngineMotion::Down,
		EditorMotion::LineStart => EngineMotion::LineStart,
		EditorMotion::LineEnd => EngineMotion::LineEnd,
	};
	publish_session(runtime, SessionRequest::ApplyEditorIntent(EditorIntent::Motion(motion)))
}

fn execute_editor_mode(runtime: &mut GlorpRuntime, mode: EditorModeCommand) -> GlorpOutcome {
	let mode = match mode {
		EditorModeCommand::EnterInsertBefore => EditorModeIntent::EnterInsertBefore,
		EditorModeCommand::EnterInsertAfter => EditorModeIntent::EnterInsertAfter,
		EditorModeCommand::ExitInsert => EditorModeIntent::ExitInsert,
	};
	publish_session(runtime, SessionRequest::ApplyEditorIntent(EditorIntent::Mode(mode)))
}

fn execute_editor_edit(runtime: &mut GlorpRuntime, edit: EditorEditIntent) -> GlorpOutcome {
	publish_session(runtime, SessionRequest::ApplyEditorIntent(EditorIntent::Edit(edit)))
}

fn execute_editor_history(runtime: &mut GlorpRuntime, action: EditorHistoryCommand) -> GlorpOutcome {
	let action = match action {
		EditorHistoryCommand::Undo => EditorHistoryIntent::Undo,
		EditorHistoryCommand::Redo => EditorHistoryIntent::Redo,
	};
	publish_session(
		runtime,
		SessionRequest::ApplyEditorIntent(EditorIntent::History(action)),
	)
}

fn execute_editor_pointer(runtime: &mut GlorpRuntime, pointer: EditorPointerIntent) -> GlorpOutcome {
	publish_session(
		runtime,
		SessionRequest::ApplyEditorIntent(EditorIntent::Pointer(pointer)),
	)
}

fn execute_ui(runtime: &mut GlorpRuntime, update: impl FnOnce(&mut crate::state::UiRuntimeState)) -> GlorpOutcome {
	update(&mut runtime.state.ui);
	publish(
		runtime,
		outcome(
			runtime.state.revisions,
			GlorpDelta {
				ui_changed: true,
				..GlorpDelta::default()
			},
			vec![],
		),
	)
}

fn execute_viewport_metrics(runtime: &mut GlorpRuntime, input: glorp_api::ViewportMetricsInput) -> GlorpOutcome {
	runtime.state.ui.layout_width = input.layout_width.max(1.0);
	runtime.state.ui.viewport_width = input.viewport_width.max(1.0);
	runtime.state.ui.viewport_height = input.viewport_height.max(1.0);
	let mut outcome = run_session(runtime, SessionRequest::SyncConfig);
	outcome.delta.ui_changed = true;
	publish(runtime, outcome)
}

fn execute_scene_ensure(runtime: &mut GlorpRuntime) -> GlorpOutcome {
	let outcome = run_session(runtime, SessionRequest::EnsureScene);
	if outcome.delta.scene_changed {
		publish(runtime, outcome)
	} else {
		outcome
	}
}

fn merge_outcome(accumulated: &mut GlorpOutcome, outcome: GlorpOutcome) {
	merge_delta(&mut accumulated.delta, &outcome.delta);
	accumulated.changed_config_paths.extend(outcome.changed_config_paths);
	accumulated.warnings.extend(outcome.warnings);
	accumulated.revisions = outcome.revisions;
}

const fn merge_delta(accumulated: &mut GlorpDelta, delta: &GlorpDelta) {
	accumulated.text_changed |= delta.text_changed;
	accumulated.view_changed |= delta.view_changed;
	accumulated.selection_changed |= delta.selection_changed;
	accumulated.mode_changed |= delta.mode_changed;
	accumulated.config_changed |= delta.config_changed;
	accumulated.ui_changed |= delta.ui_changed;
	accumulated.scene_changed |= delta.scene_changed;
}

fn run_session(runtime: &mut GlorpRuntime, request: SessionRequest) -> GlorpOutcome {
	let delta = runtime
		.state
		.session
		.execute(request, &runtime.state.config, runtime.state.ui.layout_width)
		.delta;
	session_outcome(runtime, &delta)
}

fn publish_session(runtime: &mut GlorpRuntime, request: SessionRequest) -> GlorpOutcome {
	let outcome = run_session(runtime, request);
	publish(runtime, outcome)
}

fn session_outcome(runtime: &mut GlorpRuntime, session_delta: &SessionDelta) -> GlorpOutcome {
	let delta = runtime.state.delta_from_session(session_delta);
	outcome(runtime.state.revisions, delta, vec![])
}

fn publish(runtime: &mut GlorpRuntime, outcome: GlorpOutcome) -> GlorpOutcome {
	runtime.publish_changed(&outcome);
	outcome
}

const fn outcome(revisions: GlorpRevisions, delta: GlorpDelta, changed_config_paths: Vec<ConfigPath>) -> GlorpOutcome {
	GlorpOutcome {
		delta,
		revisions,
		changed_config_paths,
		warnings: vec![],
	}
}

fn flatten_patch(value: &glorp_api::GlorpValue) -> Result<Vec<ConfigAssignment>, GlorpError> {
	flatten_patch_into("", value)
}

fn flatten_patch_into(path: &str, value: &glorp_api::GlorpValue) -> Result<Vec<ConfigAssignment>, GlorpError> {
	match value {
		glorp_api::GlorpValue::Record(fields) => {
			fields.iter().try_fold(Vec::new(), |mut assignments, (name, value)| {
				let path = match path {
					"" => name.clone(),
					_ => format!("{path}.{name}"),
				};
				assignments.extend(flatten_patch_into(&path, value)?);
				Ok(assignments)
			})
		}
		value => Ok(vec![ConfigAssignment {
			path: path.to_owned(),
			value: value.clone(),
		}]),
	}
}
