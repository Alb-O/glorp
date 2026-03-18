use {
	crate::{
		GuiCommand, project,
		runtime::GlorpRuntime,
		state::{SessionDelta, SessionRequest},
	},
	glorp_api::{
		ConfigAssignment, ConfigPath, EditorHistoryCommand, EditorModeCommand, EditorMotion, GlorpCall,
		GlorpCallResult, GlorpCallRoute, GlorpConfig, GlorpDelta, GlorpError, GlorpOutcome, GlorpRevisions,
		GlorpSubscription, GlorpTxn, TokenAckView, call_spec,
	},
	glorp_editor::{
		EditorEditIntent, EditorHistoryIntent, EditorIntent, EditorModeIntent, EditorMotion as EngineMotion,
		EditorPointerIntent,
	},
};

pub fn call(runtime: &mut GlorpRuntime, glorp_call: GlorpCall) -> Result<GlorpCallResult, GlorpError> {
	Ok(match glorp_call {
		GlorpCall::Txn(txn) => GlorpCallResult::Txn(execute_txn(runtime, txn)?),
		GlorpCall::ConfigSet(input) => GlorpCallResult::ConfigSet(execute_config_set(runtime, input)?),
		GlorpCall::ConfigReset(input) => GlorpCallResult::ConfigReset(execute_config_reset(runtime, input)?),
		GlorpCall::ConfigPatch(input) => GlorpCallResult::ConfigPatch(execute_config_patch(runtime, input)?),
		GlorpCall::ConfigReload => GlorpCallResult::ConfigReload(execute_config_reload(runtime)?),
		GlorpCall::ConfigPersist => GlorpCallResult::ConfigPersist(execute_config_persist(runtime)?),
		GlorpCall::DocumentReplace(input) => {
			GlorpCallResult::DocumentReplace(publish_session(runtime, SessionRequest::ReplaceDocument(input.text)))
		}
		GlorpCall::EditorMotion(input) => GlorpCallResult::EditorMotion(execute_editor_motion(runtime, input.motion)),
		GlorpCall::EditorMode(input) => GlorpCallResult::EditorMode(execute_editor_mode(runtime, input.mode)),
		GlorpCall::EditorInsert(input) => {
			GlorpCallResult::EditorInsert(execute_editor_edit(runtime, EditorEditIntent::InsertText(input.text)))
		}
		GlorpCall::EditorBackspace => {
			GlorpCallResult::EditorBackspace(execute_editor_edit(runtime, EditorEditIntent::Backspace))
		}
		GlorpCall::EditorDeleteForward => {
			GlorpCallResult::EditorDeleteForward(execute_editor_edit(runtime, EditorEditIntent::DeleteForward))
		}
		GlorpCall::EditorDeleteSelection => {
			GlorpCallResult::EditorDeleteSelection(execute_editor_edit(runtime, EditorEditIntent::DeleteSelection))
		}
		GlorpCall::EditorHistory(input) => {
			GlorpCallResult::EditorHistory(execute_editor_history(runtime, input.action))
		}
		GlorpCall::Schema => GlorpCallResult::Schema(glorp_api::glorp_schema()),
		GlorpCall::Config => GlorpCallResult::Config(runtime.state.config.clone()),
		GlorpCall::DocumentText => GlorpCallResult::DocumentText(runtime.state.session.text().into()),
		GlorpCall::Editor => GlorpCallResult::Editor(project::editor_view_from_state(&runtime.state)),
		GlorpCall::Capabilities => GlorpCallResult::Capabilities(capabilities()),
		GlorpCall::EventsSubscribe => {
			let token = runtime.subscriptions.subscribe(GlorpSubscription::Changes);
			GlorpCallResult::EventsSubscribe(glorp_api::GlorpEventStreamView {
				token,
				subscription: "changes".to_owned(),
			})
		}
		GlorpCall::EventsNext(input) => GlorpCallResult::EventsNext(runtime.subscriptions.next_event(input.token)?),
		GlorpCall::EventsUnsubscribe(input) => {
			runtime.subscriptions.unsubscribe(input.token)?;
			GlorpCallResult::EventsUnsubscribe(TokenAckView {
				ok: true,
				token: input.token,
			})
		}
		GlorpCall::SessionAttach => return Err(unsupported_route("session-attach", GlorpCallRoute::Client)),
		GlorpCall::SessionShutdown => return Err(unsupported_route("session-shutdown", GlorpCallRoute::Transport)),
		GlorpCall::ConfigValidate(_) => return Err(unsupported_route("config-validate", GlorpCallRoute::Client)),
	})
}

pub fn execute_gui(runtime: &mut GlorpRuntime, command: GuiCommand) -> Result<(), GlorpError> {
	match command {
		GuiCommand::SidebarSelect(tab) => execute_ui(runtime, |state| state.active_tab = tab),
		GuiCommand::InspectTargetHover(target) => execute_ui(runtime, |state| state.hovered_target = target),
		GuiCommand::InspectTargetSelect(target) => execute_ui(runtime, |state| state.selected_target = target),
		GuiCommand::CanvasFocusSet(focused) => execute_ui(runtime, |state| state.canvas_focused = focused),
		GuiCommand::ViewportScrollTo { x, y } => execute_ui(runtime, |state| {
			state.canvas_scroll_x = x.max(0.0);
			state.canvas_scroll_y = y.max(0.0);
		}),
		GuiCommand::ViewportMetricsSet {
			layout_width,
			viewport_width,
			viewport_height,
		} => publish_public_change(
			execute_viewport_metrics(runtime, layout_width, viewport_width, viewport_height),
			runtime,
		),
		GuiCommand::PaneRatioSet(ratio) => execute_ui(runtime, |state| state.pane_ratio = ratio.clamp(0.1, 0.9)),
		GuiCommand::ShowBaselinesSet(show) => execute_ui(runtime, |state| state.show_baselines = show),
		GuiCommand::ShowHitboxesSet(show) => execute_ui(runtime, |state| state.show_hitboxes = show),
		GuiCommand::EditorPointerBegin { x, y, select_word } => publish_public_change(
			execute_editor_pointer(
				runtime,
				EditorPointerIntent::Begin {
					position: point(x, y),
					select_word,
				},
			),
			runtime,
		),
		GuiCommand::EditorPointerDrag { x, y } => publish_public_change(
			execute_editor_pointer(runtime, EditorPointerIntent::Drag(point(x, y))),
			runtime,
		),
		GuiCommand::EditorPointerEnd => {
			publish_public_change(execute_editor_pointer(runtime, EditorPointerIntent::End), runtime)
		}
		GuiCommand::SceneEnsure => execute_scene_ensure(runtime),
	}

	Ok(())
}

fn execute_txn(runtime: &mut GlorpRuntime, txn: GlorpTxn) -> Result<GlorpOutcome, GlorpError> {
	let checkpoint = runtime.state.checkpoint();
	let previous_events = runtime.subscriptions_state();

	txn.calls
		.into_iter()
		.try_fold(GlorpOutcome::default(), |mut accumulated, nested_call| {
			validate_transaction_call(&nested_call)?;
			merge_outcome(&mut accumulated, call_outcome(call(runtime, nested_call)?)?);
			Ok(accumulated)
		})
		.inspect_err(|_| {
			runtime.state.restore(checkpoint);
			runtime.restore_subscriptions(previous_events);
		})
}

fn validate_transaction_call(call: &GlorpCall) -> Result<(), GlorpError> {
	if matches!(call, GlorpCall::Txn(_)) {
		return Err(GlorpError::validation(None, "nested transactions are not supported"));
	}

	let Some(spec) = call_spec(call.id()) else {
		return Err(GlorpError::not_found(format!("unknown call `{}`", call.id())));
	};

	if !spec.transactional {
		return Err(GlorpError::validation(
			None,
			format!("call `{}` is not allowed inside `txn`", spec.id),
		));
	}

	Ok(())
}

fn call_outcome(result: GlorpCallResult) -> Result<GlorpOutcome, GlorpError> {
	result.into_outcome().map_err(|other| {
		GlorpError::internal(format!(
			"transactional call returned non-outcome payload for `{}`",
			other.id()
		))
	})
}

fn capabilities() -> glorp_api::GlorpCapabilities {
	glorp_api::GlorpCapabilities {
		transactions: true,
		subscriptions: true,
		transports: vec!["local".into(), "ipc".into()],
	}
}

fn unsupported_route(id: &str, route: GlorpCallRoute) -> GlorpError {
	GlorpError::validation(
		None,
		format!("call `{id}` must be handled by the {route:?} route").to_lowercase(),
	)
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

fn execute_ui(runtime: &mut GlorpRuntime, update: impl FnOnce(&mut crate::state::UiRuntimeState)) {
	update(&mut runtime.state.ui);
}

fn publish_public_change(outcome: GlorpOutcome, runtime: &mut GlorpRuntime) {
	if public_delta_changed(&outcome.delta) {
		runtime.publish_changed(&outcome);
	}
}

fn execute_viewport_metrics(
	runtime: &mut GlorpRuntime, layout_width: f32, viewport_width: f32, viewport_height: f32,
) -> GlorpOutcome {
	runtime.state.ui.layout_width = layout_width.max(1.0);
	runtime.state.ui.viewport_width = viewport_width.max(1.0);
	runtime.state.ui.viewport_height = viewport_height.max(1.0);
	run_session(runtime, SessionRequest::SyncConfig)
}

fn execute_scene_ensure(runtime: &mut GlorpRuntime) {
	project::ensure_scene_materialized(&mut runtime.state);
}

const fn point(x: f32, y: f32) -> iced::Point {
	iced::Point::new(x, y)
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
}

fn run_session(runtime: &mut GlorpRuntime, request: SessionRequest) -> GlorpOutcome {
	let delta = runtime
		.state
		.session
		.execute(request, &runtime.state.config, runtime.state.ui.layout_width);
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

const fn public_delta_changed(delta: &GlorpDelta) -> bool {
	delta.text_changed || delta.view_changed || delta.selection_changed || delta.mode_changed || delta.config_changed
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
