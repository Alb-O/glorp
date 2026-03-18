use {
	crate::{
		GlorpError, GlorpExec, GlorpHelper, GlorpQuery, GlorpValue, OperationKind, OperationSpec, SchemaType, TypeRef,
	},
	serde::de::DeserializeOwned,
	std::sync::LazyLock,
};

static OPERATION_SPECS: LazyLock<Vec<OperationSpec>> = LazyLock::new(|| {
	vec![
		exec(
			"txn",
			"Apply multiple exec operations atomically.",
			Some(named::<crate::GlorpTxn>()),
			named::<crate::GlorpOutcome>(),
		),
		exec(
			"config-set",
			"Set one config field.",
			Some(named::<crate::ConfigAssignment>()),
			named::<crate::GlorpOutcome>(),
		),
		exec(
			"config-reset",
			"Reset one config field to its default.",
			Some(named::<crate::ConfigPathInput>()),
			named::<crate::GlorpOutcome>(),
		),
		exec(
			"config-patch",
			"Patch config with a record.",
			Some(named::<crate::ConfigPatchInput>()),
			named::<crate::GlorpOutcome>(),
		),
		exec(
			"config-reload",
			"Reload the durable config file.",
			None,
			named::<crate::GlorpOutcome>(),
		),
		exec(
			"config-persist",
			"Persist the effective config file.",
			None,
			named::<crate::GlorpOutcome>(),
		),
		exec(
			"document-replace",
			"Replace the document text.",
			Some(named::<crate::TextInput>()),
			named::<crate::GlorpOutcome>(),
		),
		exec(
			"editor-motion",
			"Apply a typed editor motion.",
			Some(named::<crate::EditorMotionInput>()),
			named::<crate::GlorpOutcome>(),
		),
		exec(
			"editor-mode",
			"Apply a typed editor mode change.",
			Some(named::<crate::EditorModeInput>()),
			named::<crate::GlorpOutcome>(),
		),
		exec(
			"editor-insert",
			"Insert text at the current selection.",
			Some(named::<crate::TextInput>()),
			named::<crate::GlorpOutcome>(),
		),
		exec(
			"editor-backspace",
			"Delete backward.",
			None,
			named::<crate::GlorpOutcome>(),
		),
		exec(
			"editor-delete-forward",
			"Delete forward.",
			None,
			named::<crate::GlorpOutcome>(),
		),
		exec(
			"editor-delete-selection",
			"Delete the current selection.",
			None,
			named::<crate::GlorpOutcome>(),
		),
		exec(
			"editor-history",
			"Apply editor history navigation.",
			Some(named::<crate::EditorHistoryInput>()),
			named::<crate::GlorpOutcome>(),
		),
		query(
			"schema",
			"Return the protocol reflection schema.",
			None,
			named::<crate::GlorpSchema>(),
		),
		query(
			"config",
			"Return the effective runtime config.",
			None,
			named::<crate::GlorpConfig>(),
		),
		query(
			"document-text",
			"Return the current document text.",
			None,
			TypeRef::Builtin(crate::BuiltinType::String),
		),
		query(
			"editor",
			"Return the current editor read model.",
			None,
			named::<crate::EditorStateView>(),
		),
		query(
			"capabilities",
			"Return stable runtime capability flags.",
			None,
			named::<crate::GlorpCapabilities>(),
		),
		helper(
			"session-attach",
			"Resolve and validate a live Glorp session.",
			None,
			named::<crate::GlorpSessionView>(),
		),
		helper(
			"session-shutdown",
			"Stop the live shared runtime for the resolved session.",
			None,
			named::<crate::OkView>(),
		),
		helper(
			"config-validate",
			"Validate a config value without mutating runtime state.",
			Some(named::<crate::ConfigAssignment>()),
			named::<crate::OkView>(),
		),
		helper(
			"events-subscribe",
			"Subscribe to runtime change events.",
			None,
			named::<crate::GlorpEventStreamView>(),
		),
		helper(
			"events-next",
			"Poll the next event for a subscription token.",
			Some(named::<crate::StreamTokenInput>()),
			Option::<crate::GlorpEvent>::type_ref(),
		),
		helper(
			"events-unsubscribe",
			"Release a subscription token.",
			Some(named::<crate::StreamTokenInput>()),
			named::<crate::TokenAckView>(),
		),
	]
});

pub fn operation_specs() -> &'static [OperationSpec] {
	OPERATION_SPECS.as_slice()
}

pub fn exec_ids() -> Vec<String> {
	operation_ids(OperationKind::Exec)
}

pub fn query_ids() -> Vec<String> {
	operation_ids(OperationKind::Query)
}

pub fn helper_ids() -> Vec<String> {
	operation_ids(OperationKind::Helper)
}

pub fn build_exec(id: &str, input: Option<&GlorpValue>) -> Result<GlorpExec, GlorpError> {
	Ok(match id {
		"txn" => GlorpExec::Txn(decode_required(id, input)?),
		"config-set" => GlorpExec::ConfigSet(decode_required(id, input)?),
		"config-reset" => GlorpExec::ConfigReset(decode_required(id, input)?),
		"config-patch" => GlorpExec::ConfigPatch(decode_required(id, input)?),
		"config-reload" => {
			ensure_no_input(id, input)?;
			GlorpExec::ConfigReload
		}
		"config-persist" => {
			ensure_no_input(id, input)?;
			GlorpExec::ConfigPersist
		}
		"document-replace" => GlorpExec::DocumentReplace(decode_required(id, input)?),
		"editor-motion" => GlorpExec::EditorMotion(decode_required(id, input)?),
		"editor-mode" => GlorpExec::EditorMode(decode_required(id, input)?),
		"editor-insert" => GlorpExec::EditorInsert(decode_required(id, input)?),
		"editor-backspace" => {
			ensure_no_input(id, input)?;
			GlorpExec::EditorBackspace
		}
		"editor-delete-forward" => {
			ensure_no_input(id, input)?;
			GlorpExec::EditorDeleteForward
		}
		"editor-delete-selection" => {
			ensure_no_input(id, input)?;
			GlorpExec::EditorDeleteSelection
		}
		"editor-history" => GlorpExec::EditorHistory(decode_required(id, input)?),
		_ => return Err(unknown_operation("exec", id)),
	})
}

pub fn build_query(id: &str, input: Option<&GlorpValue>) -> Result<GlorpQuery, GlorpError> {
	Ok(match id {
		"schema" => {
			ensure_no_input(id, input)?;
			GlorpQuery::Schema
		}
		"config" => {
			ensure_no_input(id, input)?;
			GlorpQuery::Config
		}
		"document-text" => {
			ensure_no_input(id, input)?;
			GlorpQuery::DocumentText
		}
		"editor" => {
			ensure_no_input(id, input)?;
			GlorpQuery::Editor
		}
		"capabilities" => {
			ensure_no_input(id, input)?;
			GlorpQuery::Capabilities
		}
		_ => return Err(unknown_operation("query", id)),
	})
}

pub fn build_helper(id: &str, input: Option<&GlorpValue>) -> Result<GlorpHelper, GlorpError> {
	Ok(match id {
		"session-attach" => {
			ensure_no_input(id, input)?;
			GlorpHelper::SessionAttach
		}
		"session-shutdown" => {
			ensure_no_input(id, input)?;
			GlorpHelper::SessionShutdown
		}
		"config-validate" => GlorpHelper::ConfigValidate(decode_required(id, input)?),
		"events-subscribe" => {
			ensure_no_input(id, input)?;
			GlorpHelper::EventsSubscribe
		}
		"events-next" => GlorpHelper::EventsNext(decode_required(id, input)?),
		"events-unsubscribe" => GlorpHelper::EventsUnsubscribe(decode_required(id, input)?),
		_ => return Err(unknown_operation("helper", id)),
	})
}

pub fn render_nu_completions() -> String {
	[
		render_completion("exec-op", &exec_ids()),
		render_completion("query-op", &query_ids()),
		render_completion("helper-op", &helper_ids()),
	]
	.join("")
}

pub fn render_nu_module() -> String {
	"# source this file after registering `nu_plugin_glorp` with `plugin add`\nplugin use glorp\nuse ./completions.nu *\n".to_owned()
}

fn named<T>() -> TypeRef
where
	T: SchemaType, {
	T::type_ref()
}

fn exec(id: &str, docs: &str, input: Option<TypeRef>, output: TypeRef) -> OperationSpec {
	operation(id, OperationKind::Exec, docs, input, output)
}

fn query(id: &str, docs: &str, input: Option<TypeRef>, output: TypeRef) -> OperationSpec {
	operation(id, OperationKind::Query, docs, input, output)
}

fn helper(id: &str, docs: &str, input: Option<TypeRef>, output: TypeRef) -> OperationSpec {
	operation(id, OperationKind::Helper, docs, input, output)
}

fn operation(id: &str, kind: OperationKind, docs: &str, input: Option<TypeRef>, output: TypeRef) -> OperationSpec {
	OperationSpec {
		id: id.to_owned(),
		kind,
		docs: docs.to_owned(),
		input,
		output,
	}
}

fn ensure_no_input(id: &str, input: Option<&GlorpValue>) -> Result<(), GlorpError> {
	match input {
		None | Some(GlorpValue::Null) => Ok(()),
		Some(_) => Err(GlorpError::validation(
			None,
			format!("operation `{id}` does not accept input"),
		)),
	}
}

fn decode_required<T>(id: &str, input: Option<&GlorpValue>) -> Result<T, GlorpError>
where
	T: DeserializeOwned, {
	let Some(input) = input else {
		return Err(GlorpError::validation(None, format!("operation `{id}` requires input")));
	};

	serde_json::from_value::<T>(input.into())
		.map_err(|error| GlorpError::validation(None, format!("invalid input for `{id}`: {error}")))
}

fn unknown_operation(kind: &str, id: &str) -> GlorpError {
	GlorpError::not_found(format!("unknown {kind} operation `{id}`"))
}

fn operation_ids(kind: OperationKind) -> Vec<String> {
	operation_specs()
		.iter()
		.filter(|operation| operation.kind == kind)
		.map(|operation| operation.id.clone())
		.collect()
}

fn render_completion(name: &str, values: &[String]) -> String {
	let values = values
		.iter()
		.map(|value| format!("\"{value}\""))
		.collect::<Vec<_>>()
		.join(" ");
	format!("export def \"nu-complete glorp {name}\" [] {{ [{values}] }}\n")
}
