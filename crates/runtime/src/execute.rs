use {
	crate::{
		GuiDocumentSyncReason, GuiDocumentSyncRef, GuiEditCommand, GuiEditRequest, GuiEditResponse, project,
		runtime::GlorpRuntime,
		state::{SessionDelta, SessionRequest},
	},
	glorp_api::{
		ConfigAssignment, ConfigPath, GlorpCall, GlorpCallResult, GlorpConfig, GlorpDelta, GlorpError, GlorpOutcome,
		GlorpRevisions, GlorpSubscription, GlorpTxn, RuntimeCallDispatcher, TextRange, TokenAckView,
		decode_call_output, dispatch_runtime_call, transactional_call_spec,
	},
	glorp_editor::TextEdit,
};

pub fn call(runtime: &mut GlorpRuntime, glorp_call: GlorpCall) -> Result<GlorpCallResult, GlorpError> {
	dispatch_runtime_call(runtime, glorp_call)
}

pub fn execute_gui_edit(runtime: &mut GlorpRuntime, request: GuiEditRequest) -> Result<GuiEditResponse, GlorpError> {
	let (undo_depth, redo_depth) = runtime.state.session.history_depths();
	if request.base_revision != runtime.state.revisions.editor {
		return Ok(GuiEditResponse::RejectedStale {
			latest_revision: runtime.state.revisions.editor,
			undo_depth,
			redo_depth,
		});
	}

	let session_request = match request.command {
		GuiEditCommand::ReplaceRange { range, inserted } => {
			SessionRequest::ApplyEdit(validate_text_edit(runtime.state.session.text(), range, inserted)?)
		}
		GuiEditCommand::Undo => SessionRequest::Undo,
		GuiEditCommand::Redo => SessionRequest::Redo,
	};
	let outcome = publish_session(runtime, session_request);
	let (undo_depth, redo_depth) = runtime.state.session.history_depths();
	Ok(GuiEditResponse::Applied {
		revisions: outcome.revisions,
		outcome: private_outcome(
			&outcome,
			private_document_sync_ref(&outcome, GuiDocumentSyncReason::LargeEdit).is_some(),
		),
		undo_depth,
		redo_depth,
	})
}

fn execute_txn(runtime: &mut GlorpRuntime, txn: GlorpTxn) -> Result<GlorpOutcome, GlorpError> {
	let checkpoint = runtime.state.checkpoint();
	let previous_events = runtime.subscriptions_state();

	txn.calls
		.into_iter()
		.try_fold(GlorpOutcome::default(), |mut accumulated, nested_call| {
			transactional_call_spec(&nested_call)?;
			merge_outcome(
				&mut accumulated,
				call_outcome(dispatch_runtime_call(runtime, nested_call)?)?,
			);
			Ok(accumulated)
		})
		.inspect_err(|_| {
			runtime.state.restore(checkpoint);
			runtime.restore_subscriptions(previous_events);
		})
}

fn call_outcome(result: GlorpCallResult) -> Result<GlorpOutcome, GlorpError> {
	let id = result.id.clone();
	decode_call_output::<GlorpOutcome>(&id, &result.output)
		.map_err(|_| GlorpError::internal(format!("transactional call returned non-outcome payload for `{}`", id)))
}

fn capabilities() -> glorp_api::GlorpCapabilities {
	glorp_api::GlorpCapabilities {
		transactions: true,
		subscriptions: true,
		streaming: true,
		binary_payloads: true,
		transports: vec!["local".into(), "ipc".into()],
	}
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
	runtime.state.revisions.config += 1;
	let outcome = GlorpOutcome {
		delta: GlorpDelta {
			text_changed: false,
			view_changed: false,
			config_changed: !changed_paths.is_empty(),
		},
		revisions: runtime.state.revisions,
		document_edit: None,
		changed_config_paths: changed_paths,
		warnings: vec![],
	};
	publish_change_streams(runtime, &outcome);
	Ok(outcome)
}

fn merge_outcome(accumulated: &mut GlorpOutcome, outcome: GlorpOutcome) {
	merge_delta(&mut accumulated.delta, &outcome.delta);
	accumulated.changed_config_paths.extend(outcome.changed_config_paths);
	accumulated.warnings.extend(outcome.warnings);
	accumulated.document_edit = outcome.document_edit.or_else(|| accumulated.document_edit.take());
	accumulated.revisions = outcome.revisions;
}

const fn merge_delta(accumulated: &mut GlorpDelta, delta: &GlorpDelta) {
	accumulated.text_changed |= delta.text_changed;
	accumulated.view_changed |= delta.view_changed;
	accumulated.config_changed |= delta.config_changed;
}

fn run_session(runtime: &mut GlorpRuntime, request: SessionRequest) -> GlorpOutcome {
	let delta = runtime.state.session.execute(request);
	session_outcome(runtime, delta)
}

fn publish_session(runtime: &mut GlorpRuntime, request: SessionRequest) -> GlorpOutcome {
	let outcome = run_session(runtime, request);
	publish(runtime, outcome)
}

fn session_outcome(runtime: &mut GlorpRuntime, session_delta: SessionDelta) -> GlorpOutcome {
	let delta = runtime.state.delta_from_session(&session_delta);
	outcome(
		runtime.state.revisions,
		delta,
		session_delta.document_edit.map(crate::state::text_edit_view),
		vec![],
	)
}

fn publish(runtime: &mut GlorpRuntime, outcome: GlorpOutcome) -> GlorpOutcome {
	if public_delta_changed(&outcome.delta) {
		publish_change_streams(runtime, &outcome);
	}
	outcome
}

const fn public_delta_changed(delta: &GlorpDelta) -> bool {
	delta.text_changed || delta.view_changed || delta.config_changed
}

fn outcome(
	revisions: GlorpRevisions, delta: GlorpDelta, document_edit: Option<glorp_api::TextEditView>,
	changed_config_paths: Vec<ConfigPath>,
) -> GlorpOutcome {
	GlorpOutcome {
		delta,
		revisions,
		document_edit,
		changed_config_paths,
		warnings: vec![],
	}
}

pub fn gui_shared_delta(runtime: &GlorpRuntime, outcome: &GlorpOutcome) -> crate::GuiSharedDelta {
	let (undo_depth, redo_depth) = runtime.state.session.history_depths();
	let document_sync = private_document_sync_ref(&outcome, GuiDocumentSyncReason::LargeEdit);
	crate::GuiSharedDelta {
		undo_depth,
		redo_depth,
		config: outcome.delta.config_changed.then(|| runtime.state.config.clone()),
		document_sync,
		outcome: private_outcome(outcome, document_sync.is_some()),
	}
}

pub fn document_sync_ref(revision: u64, text: &str, reason: GuiDocumentSyncReason) -> Option<GuiDocumentSyncRef> {
	(text.len() > crate::LARGE_PAYLOAD_BYTES).then_some(GuiDocumentSyncRef {
		revision,
		bytes: text.len(),
		reason,
	})
}

fn private_document_sync_ref(outcome: &GlorpOutcome, reason: GuiDocumentSyncReason) -> Option<GuiDocumentSyncRef> {
	let edit = outcome.document_edit.as_ref()?;
	document_sync_ref(outcome.revisions.editor, edit.inserted.as_str(), reason)
}

fn private_outcome(outcome: &GlorpOutcome, strip_document_edit: bool) -> GlorpOutcome {
	GlorpOutcome {
		delta: outcome.delta.clone(),
		revisions: outcome.revisions,
		document_edit: (!strip_document_edit).then(|| outcome.document_edit.clone()).flatten(),
		changed_config_paths: outcome.changed_config_paths.clone(),
		warnings: outcome.warnings.clone(),
	}
}

fn publish_change_streams(runtime: &mut GlorpRuntime, outcome: &GlorpOutcome) {
	let gui_delta = gui_shared_delta(runtime, outcome);
	runtime.publish_gui_changed(&gui_delta);
	runtime.publish_changed(outcome);
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

fn validate_text_edit(text: &str, range: TextRange, inserted: String) -> Result<TextEdit, GlorpError> {
	let range = range.start as usize..range.end as usize;
	if range.start > range.end || range.end > text.len() {
		return Err(GlorpError::validation(None, "text edit range is out of bounds"));
	}
	if !text.is_char_boundary(range.start) || !text.is_char_boundary(range.end) {
		return Err(GlorpError::validation(
			None,
			"text edit range must stay on UTF-8 boundaries",
		));
	}
	Ok(TextEdit { range, inserted })
}

impl RuntimeCallDispatcher for GlorpRuntime {
	fn txn(&mut self, input: GlorpTxn) -> Result<GlorpOutcome, GlorpError> {
		execute_txn(self, input)
	}

	fn config_set(&mut self, input: ConfigAssignment) -> Result<GlorpOutcome, GlorpError> {
		execute_config_set(self, input)
	}

	fn config_reset(&mut self, input: glorp_api::ConfigPathInput) -> Result<GlorpOutcome, GlorpError> {
		execute_config_reset(self, input)
	}

	fn config_patch(&mut self, input: glorp_api::ConfigPatchInput) -> Result<GlorpOutcome, GlorpError> {
		execute_config_patch(self, input)
	}

	fn config_reload(&mut self, _input: ()) -> Result<GlorpOutcome, GlorpError> {
		execute_config_reload(self)
	}

	fn config_persist(&mut self, _input: ()) -> Result<GlorpOutcome, GlorpError> {
		execute_config_persist(self)
	}

	fn document_replace(&mut self, input: glorp_api::TextInput) -> Result<GlorpOutcome, GlorpError> {
		Ok(publish_session(self, SessionRequest::ReplaceDocument(input.text)))
	}

	fn schema(&mut self, _input: ()) -> Result<glorp_api::GlorpSchema, GlorpError> {
		Ok(glorp_api::glorp_schema())
	}

	fn config(&mut self, _input: ()) -> Result<GlorpConfig, GlorpError> {
		Ok(self.state.config.clone())
	}

	fn document_text(&mut self, _input: ()) -> Result<String, GlorpError> {
		Ok(self.state.session.text().into())
	}

	fn document(&mut self, _input: ()) -> Result<glorp_api::DocumentStateView, GlorpError> {
		Ok(project::document_view_from_state(&self.state))
	}

	fn capabilities(&mut self, _input: ()) -> Result<glorp_api::GlorpCapabilities, GlorpError> {
		Ok(capabilities())
	}

	fn events_subscribe(&mut self, _input: ()) -> Result<glorp_api::GlorpEventStreamView, GlorpError> {
		let token = self.subscriptions.subscribe(GlorpSubscription::Changes);
		Ok(glorp_api::GlorpEventStreamView {
			token,
			subscription: "changes".to_owned(),
		})
	}

	fn events_next(&mut self, input: glorp_api::StreamTokenInput) -> Result<Option<glorp_api::GlorpEvent>, GlorpError> {
		self.subscriptions.next_event(input.token)
	}

	fn events_unsubscribe(&mut self, input: glorp_api::StreamTokenInput) -> Result<TokenAckView, GlorpError> {
		self.subscriptions.unsubscribe(input.token)?;
		Ok(TokenAckView {
			ok: true,
			token: input.token,
		})
	}
}
