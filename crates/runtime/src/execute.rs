use {
	crate::{runtime::GlorpRuntime, state::SessionRequest},
	glorp_api::*,
	glorp_editor::{
		EditorEditIntent, EditorHistoryIntent, EditorIntent, EditorModeIntent, EditorMotion as EngineMotion,
		EditorPointerIntent,
	},
};

pub fn execute(runtime: &mut GlorpRuntime, command: GlorpCommand) -> Result<GlorpOutcome, GlorpError> {
	match command {
		GlorpCommand::Txn(txn) => execute_txn(runtime, txn),
		GlorpCommand::Config(command) => execute_config(runtime, command),
		GlorpCommand::Document(command) => execute_document(runtime, command),
		GlorpCommand::Editor(command) => execute_editor(runtime, command),
		GlorpCommand::Ui(command) => execute_ui(runtime, command),
		GlorpCommand::Scene(command) => execute_scene(runtime, command),
	}
}

fn execute_txn(runtime: &mut GlorpRuntime, txn: GlorpTxn) -> Result<GlorpOutcome, GlorpError> {
	let checkpoint = runtime.state.checkpoint();
	let previous_events = runtime.subscriptions_state();
	let mut accumulated = GlorpOutcome::default();

	for command in txn.commands {
		match execute(runtime, command) {
			Ok(outcome) => {
				accumulated.delta.text_changed |= outcome.delta.text_changed;
				accumulated.delta.view_changed |= outcome.delta.view_changed;
				accumulated.delta.selection_changed |= outcome.delta.selection_changed;
				accumulated.delta.mode_changed |= outcome.delta.mode_changed;
				accumulated.delta.config_changed |= outcome.delta.config_changed;
				accumulated.delta.ui_changed |= outcome.delta.ui_changed;
				accumulated.delta.scene_changed |= outcome.delta.scene_changed;
				accumulated.changed_config_paths.extend(outcome.changed_config_paths);
				accumulated.warnings.extend(outcome.warnings);
				accumulated.revisions = outcome.revisions;
			}
			Err(error) => {
				runtime.state.restore(checkpoint);
				runtime.restore_subscriptions(previous_events);
				return Err(error);
			}
		}
	}

	Ok(accumulated)
}

fn execute_config(runtime: &mut GlorpRuntime, command: ConfigCommand) -> Result<GlorpOutcome, GlorpError> {
	let mut changed_paths = Vec::new();

	match command {
		ConfigCommand::Set { path, value } => {
			runtime.state.config.set_path(&path, value)?;
			changed_paths.push(path);
		}
		ConfigCommand::Patch { values } => {
			changed_paths = runtime.state.config.patch(&values)?;
		}
		ConfigCommand::Reset { path } => {
			runtime.state.config.reset_path(&path)?;
			changed_paths.push(path);
		}
		ConfigCommand::Reload => {
			runtime.state.config = runtime.config_store.load()?;
			changed_paths = GlorpConfig::schema_defaults()
				.into_iter()
				.map(|(path, _)| path)
				.collect();
		}
		ConfigCommand::Persist => {
			runtime.config_store.save(&runtime.state.config)?;
			return Ok(GlorpOutcome {
				delta: GlorpDelta::default(),
				revisions: runtime.state.revisions,
				changed_config_paths: Vec::new(),
				warnings: Vec::new(),
			});
		}
	}

	let session = runtime.state.session.execute(
		SessionRequest::SyncConfig,
		&runtime.state.config,
		runtime.state.ui.layout_width,
	);

	runtime.state.revisions.config += 1;
	let mut delta = runtime.state.delta_from_session(&session.delta);
	delta.config_changed = !changed_paths.is_empty();
	let outcome = GlorpOutcome {
		delta,
		revisions: runtime.state.revisions,
		changed_config_paths: changed_paths,
		warnings: Vec::new(),
	};
	runtime.publish_changed(&outcome);
	Ok(outcome)
}

fn execute_document(runtime: &mut GlorpRuntime, command: DocumentCommand) -> Result<GlorpOutcome, GlorpError> {
	match command {
		DocumentCommand::Replace { text } => {
			let session = runtime.state.session.execute(
				SessionRequest::ReplaceDocument(text),
				&runtime.state.config,
				runtime.state.ui.layout_width,
			);
			let outcome = GlorpOutcome {
				delta: runtime.state.delta_from_session(&session.delta),
				revisions: runtime.state.revisions,
				changed_config_paths: Vec::new(),
				warnings: Vec::new(),
			};
			runtime.publish_changed(&outcome);
			Ok(outcome)
		}
	}
}

fn execute_editor(runtime: &mut GlorpRuntime, command: EditorCommand) -> Result<GlorpOutcome, GlorpError> {
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

	let session = runtime.state.session.execute(
		SessionRequest::ApplyEditorIntent(intent),
		&runtime.state.config,
		runtime.state.ui.layout_width,
	);
	let outcome = GlorpOutcome {
		delta: runtime.state.delta_from_session(&session.delta),
		revisions: runtime.state.revisions,
		changed_config_paths: Vec::new(),
		warnings: Vec::new(),
	};
	runtime.publish_changed(&outcome);
	Ok(outcome)
}

fn execute_ui(runtime: &mut GlorpRuntime, command: UiCommand) -> Result<GlorpOutcome, GlorpError> {
	match command {
		UiCommand::SidebarSelect { tab } => runtime.state.ui.active_tab = tab,
		UiCommand::InspectTargetSelect { target } => runtime.state.ui.selected_target = target,
		UiCommand::ViewportScrollTo { x, y } => {
			runtime.state.ui.canvas_scroll_x = x.max(0.0);
			runtime.state.ui.canvas_scroll_y = y.max(0.0);
		}
		UiCommand::PaneRatioSet { ratio } => runtime.state.ui.pane_ratio = ratio.clamp(0.1, 0.9),
	}

	let outcome = GlorpOutcome {
		delta: GlorpDelta {
			ui_changed: true,
			..GlorpDelta::default()
		},
		revisions: runtime.state.revisions,
		changed_config_paths: Vec::new(),
		warnings: Vec::new(),
	};
	runtime.publish_changed(&outcome);
	Ok(outcome)
}

fn execute_scene(runtime: &mut GlorpRuntime, command: SceneCommand) -> Result<GlorpOutcome, GlorpError> {
	match command {
		SceneCommand::Ensure => {
			let session = runtime.state.session.execute(
				SessionRequest::EnsureScene,
				&runtime.state.config,
				runtime.state.ui.layout_width,
			);
			let outcome = GlorpOutcome {
				delta: runtime.state.delta_from_session(&session.delta),
				revisions: runtime.state.revisions,
				changed_config_paths: Vec::new(),
				warnings: Vec::new(),
			};
			if outcome.delta.scene_changed {
				runtime.publish_changed(&outcome);
			}
			Ok(outcome)
		}
	}
}
