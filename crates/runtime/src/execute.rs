use {
	crate::{
		runtime::GlorpRuntime,
		state::{SessionDelta, SessionRequest},
	},
	glorp_api::{
		ConfigCommand, ConfigPath, DocumentCommand, EditorCommand, EditorEditCommand, EditorHistoryCommand,
		EditorModeCommand, EditorMotion, EditorPointerCommand, GlorpCommand, GlorpConfig, GlorpDelta, GlorpError,
		GlorpOutcome, GlorpRevisions, GlorpTxn, SceneCommand, UiCommand,
	},
	glorp_editor::{
		EditorEditIntent, EditorHistoryIntent, EditorIntent, EditorModeIntent, EditorMotion as EngineMotion,
		EditorPointerIntent,
	},
};

pub fn execute(runtime: &mut GlorpRuntime, command: GlorpCommand) -> Result<GlorpOutcome, GlorpError> {
	match command {
		GlorpCommand::Txn(txn) => execute_txn(runtime, txn),
		GlorpCommand::Config(command) => execute_config(runtime, command),
		GlorpCommand::Document(command) => Ok(execute_document(runtime, command)),
		GlorpCommand::Editor(command) => Ok(execute_editor(runtime, command)),
		GlorpCommand::Ui(command) => Ok(execute_ui(runtime, &command)),
		GlorpCommand::Scene(command) => Ok(execute_scene(runtime, command)),
	}
}

fn execute_txn(runtime: &mut GlorpRuntime, txn: GlorpTxn) -> Result<GlorpOutcome, GlorpError> {
	let checkpoint = runtime.state.checkpoint();
	let previous_events = runtime.subscriptions_state();

	txn.commands
		.into_iter()
		.try_fold(GlorpOutcome::default(), |mut accumulated, command| {
			merge_outcome(&mut accumulated, execute(runtime, command)?);
			Ok(accumulated)
		})
		.inspect_err(|_| {
			runtime.state.restore(checkpoint);
			runtime.restore_subscriptions(previous_events);
		})
}

fn execute_config(runtime: &mut GlorpRuntime, command: ConfigCommand) -> Result<GlorpOutcome, GlorpError> {
	let changed_paths = match command {
		ConfigCommand::Set { path, value } => {
			runtime.state.config.set_path(&path, value)?;
			vec![path]
		}
		ConfigCommand::Patch { values } => runtime.state.config.patch(&values)?,
		ConfigCommand::Reset { path } => {
			runtime.state.config.reset_path(&path)?;
			vec![path]
		}
		ConfigCommand::Reload => {
			runtime.state.config = runtime.config_store.load()?;
			GlorpConfig::schema_defaults()
				.into_iter()
				.map(|(path, _)| path)
				.collect()
		}
		ConfigCommand::Persist => {
			runtime.config_store.save(&runtime.state.config)?;
			return Ok(GlorpOutcome {
				revisions: runtime.state.revisions,
				..GlorpOutcome::default()
			});
		}
	};

	let mut outcome = run_session(runtime, SessionRequest::SyncConfig);
	runtime.state.revisions.config += 1;
	outcome.revisions = runtime.state.revisions;
	outcome.delta.config_changed = !changed_paths.is_empty();
	outcome.changed_config_paths = changed_paths;
	runtime.publish_changed(&outcome);
	Ok(outcome)
}

fn execute_document(runtime: &mut GlorpRuntime, command: DocumentCommand) -> GlorpOutcome {
	match command {
		DocumentCommand::Replace { text } => publish_session(runtime, SessionRequest::ReplaceDocument(text)),
	}
}

fn execute_editor(runtime: &mut GlorpRuntime, command: EditorCommand) -> GlorpOutcome {
	let intent = match command {
		EditorCommand::Motion(motion) => EditorIntent::Motion(match motion {
			EditorMotion::Left => EngineMotion::Left,
			EditorMotion::Right => EngineMotion::Right,
			EditorMotion::Up => EngineMotion::Up,
			EditorMotion::Down => EngineMotion::Down,
			EditorMotion::LineStart => EngineMotion::LineStart,
			EditorMotion::LineEnd => EngineMotion::LineEnd,
		}),
		EditorCommand::Mode(mode) => EditorIntent::Mode(match mode {
			EditorModeCommand::EnterInsertBefore => EditorModeIntent::EnterInsertBefore,
			EditorModeCommand::EnterInsertAfter => EditorModeIntent::EnterInsertAfter,
			EditorModeCommand::ExitInsert => EditorModeIntent::ExitInsert,
		}),
		EditorCommand::Edit(edit) => EditorIntent::Edit(match edit {
			EditorEditCommand::Backspace => EditorEditIntent::Backspace,
			EditorEditCommand::DeleteForward => EditorEditIntent::DeleteForward,
			EditorEditCommand::DeleteSelection => EditorEditIntent::DeleteSelection,
			EditorEditCommand::Insert { text } => EditorEditIntent::InsertText(text),
		}),
		EditorCommand::History(history) => EditorIntent::History(match history {
			EditorHistoryCommand::Undo => EditorHistoryIntent::Undo,
			EditorHistoryCommand::Redo => EditorHistoryIntent::Redo,
		}),
		EditorCommand::Pointer(pointer) => EditorIntent::Pointer(match pointer {
			EditorPointerCommand::Begin { x, y, select_word } => EditorPointerIntent::Begin {
				position: iced::Point::new(x, y),
				select_word,
			},
			EditorPointerCommand::Drag { x, y } => EditorPointerIntent::Drag(iced::Point::new(x, y)),
			EditorPointerCommand::End => EditorPointerIntent::End,
		}),
	};

	publish_session(runtime, SessionRequest::ApplyEditorIntent(intent))
}

fn execute_ui(runtime: &mut GlorpRuntime, command: &UiCommand) -> GlorpOutcome {
	match command {
		UiCommand::SidebarSelect { tab } => runtime.state.ui.active_tab = *tab,
		UiCommand::InspectTargetHover { target } => runtime.state.ui.hovered_target = *target,
		UiCommand::InspectTargetSelect { target } => runtime.state.ui.selected_target = *target,
		UiCommand::CanvasFocusSet { focused } => runtime.state.ui.canvas_focused = *focused,
		UiCommand::ViewportScrollTo { x, y } => {
			runtime.state.ui.canvas_scroll_x = x.max(0.0);
			runtime.state.ui.canvas_scroll_y = y.max(0.0);
		}
		UiCommand::ViewportMetricsSet {
			layout_width,
			viewport_width,
			viewport_height,
		} => {
			runtime.state.ui.layout_width = layout_width.max(1.0);
			runtime.state.ui.viewport_width = viewport_width.max(1.0);
			runtime.state.ui.viewport_height = viewport_height.max(1.0);
			let mut outcome = run_session(runtime, SessionRequest::SyncConfig);
			outcome.delta.ui_changed = true;
			return publish(runtime, outcome);
		}
		UiCommand::PaneRatioSet { ratio } => runtime.state.ui.pane_ratio = ratio.clamp(0.1, 0.9),
	}

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

fn execute_scene(runtime: &mut GlorpRuntime, command: SceneCommand) -> GlorpOutcome {
	match command {
		SceneCommand::Ensure => {
			let outcome = run_session(runtime, SessionRequest::EnsureScene);
			if outcome.delta.scene_changed {
				publish(runtime, outcome)
			} else {
				outcome
			}
		}
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
