#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NamedType {
	pub schema_name: &'static str,
	pub rust_ty: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CallType {
	pub rust_ty: &'static str,
	pub named_types: &'static [NamedType],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallKind {
	Mutation,
	Read,
	Helper,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallRoute {
	Runtime,
	Transport,
	Client,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CallDefinition {
	pub descriptor: &'static str,
	pub id: &'static str,
	pub handler: &'static str,
	pub docs: &'static str,
	pub kind: CallKind,
	pub route: CallRoute,
	pub transactional: bool,
	pub input: Option<CallType>,
	pub output: CallType,
}

pub const fn named_type(schema_name: &'static str, rust_ty: &'static str) -> NamedType {
	NamedType { schema_name, rust_ty }
}

pub const fn call_type(rust_ty: &'static str, named_types: &'static [NamedType]) -> CallType {
	CallType { rust_ty, named_types }
}

pub const fn call(
	descriptor: &'static str, id: &'static str, handler: &'static str, docs: &'static str, kind: CallKind,
	route: CallRoute, transactional: bool, input: Option<CallType>, output: CallType,
) -> CallDefinition {
	CallDefinition {
		descriptor,
		id,
		handler,
		docs,
		kind,
		route,
		transactional,
		input,
		output,
	}
}

pub const CALLS: &[CallDefinition] = &[
	call(
		"Txn",
		"txn",
		"txn",
		"Apply multiple mutation calls atomically.",
		CallKind::Mutation,
		CallRoute::Runtime,
		false,
		Some(call_type(
			"crate::GlorpTxn",
			&[named_type("GlorpTxn", "crate::GlorpTxn")],
		)),
		call_type(
			"crate::GlorpOutcome",
			&[named_type("GlorpOutcome", "crate::GlorpOutcome")],
		),
	),
	call(
		"ConfigSet",
		"config-set",
		"config_set",
		"Set one config field.",
		CallKind::Mutation,
		CallRoute::Runtime,
		true,
		Some(call_type(
			"crate::ConfigAssignment",
			&[named_type("ConfigAssignment", "crate::ConfigAssignment")],
		)),
		call_type(
			"crate::GlorpOutcome",
			&[named_type("GlorpOutcome", "crate::GlorpOutcome")],
		),
	),
	call(
		"ConfigReset",
		"config-reset",
		"config_reset",
		"Reset one config field to its default.",
		CallKind::Mutation,
		CallRoute::Runtime,
		true,
		Some(call_type(
			"crate::ConfigPathInput",
			&[named_type("ConfigPathInput", "crate::ConfigPathInput")],
		)),
		call_type(
			"crate::GlorpOutcome",
			&[named_type("GlorpOutcome", "crate::GlorpOutcome")],
		),
	),
	call(
		"ConfigPatch",
		"config-patch",
		"config_patch",
		"Patch config with a record.",
		CallKind::Mutation,
		CallRoute::Runtime,
		true,
		Some(call_type(
			"crate::ConfigPatchInput",
			&[named_type("ConfigPatchInput", "crate::ConfigPatchInput")],
		)),
		call_type(
			"crate::GlorpOutcome",
			&[named_type("GlorpOutcome", "crate::GlorpOutcome")],
		),
	),
	call(
		"ConfigReload",
		"config-reload",
		"config_reload",
		"Reload the durable config file.",
		CallKind::Mutation,
		CallRoute::Runtime,
		true,
		None,
		call_type(
			"crate::GlorpOutcome",
			&[named_type("GlorpOutcome", "crate::GlorpOutcome")],
		),
	),
	call(
		"ConfigPersist",
		"config-persist",
		"config_persist",
		"Persist the effective config file.",
		CallKind::Mutation,
		CallRoute::Runtime,
		true,
		None,
		call_type(
			"crate::GlorpOutcome",
			&[named_type("GlorpOutcome", "crate::GlorpOutcome")],
		),
	),
	call(
		"DocumentReplace",
		"document-replace",
		"document_replace",
		"Replace the document text.",
		CallKind::Mutation,
		CallRoute::Runtime,
		true,
		Some(call_type(
			"crate::TextInput",
			&[named_type("TextInput", "crate::TextInput")],
		)),
		call_type(
			"crate::GlorpOutcome",
			&[named_type("GlorpOutcome", "crate::GlorpOutcome")],
		),
	),
	call(
		"Schema",
		"schema",
		"schema",
		"Return the protocol reflection schema.",
		CallKind::Read,
		CallRoute::Runtime,
		false,
		None,
		call_type("crate::GlorpSchema", &[named_type("GlorpSchema", "crate::GlorpSchema")]),
	),
	call(
		"Config",
		"config",
		"config",
		"Return the effective runtime config.",
		CallKind::Read,
		CallRoute::Runtime,
		false,
		None,
		call_type("crate::GlorpConfig", &[named_type("GlorpConfig", "crate::GlorpConfig")]),
	),
	call(
		"DocumentText",
		"document-text",
		"document_text",
		"Return the current document text.",
		CallKind::Read,
		CallRoute::Runtime,
		false,
		None,
		call_type("String", &[]),
	),
	call(
		"Document",
		"document",
		"document",
		"Return the current shared document read model.",
		CallKind::Read,
		CallRoute::Runtime,
		false,
		None,
		call_type(
			"crate::DocumentStateView",
			&[named_type("DocumentStateView", "crate::DocumentStateView")],
		),
	),
	call(
		"Capabilities",
		"capabilities",
		"capabilities",
		"Return stable runtime capability flags.",
		CallKind::Read,
		CallRoute::Runtime,
		false,
		None,
		call_type(
			"crate::GlorpCapabilities",
			&[named_type("GlorpCapabilities", "crate::GlorpCapabilities")],
		),
	),
	call(
		"SessionAttach",
		"session-attach",
		"session_attach",
		"Resolve and validate a live Glorp session.",
		CallKind::Helper,
		CallRoute::Client,
		false,
		None,
		call_type(
			"crate::GlorpSessionView",
			&[named_type("GlorpSessionView", "crate::GlorpSessionView")],
		),
	),
	call(
		"SessionShutdown",
		"session-shutdown",
		"session_shutdown",
		"Stop the live shared runtime for the resolved session.",
		CallKind::Helper,
		CallRoute::Transport,
		false,
		None,
		call_type("crate::OkView", &[named_type("OkView", "crate::OkView")]),
	),
	call(
		"ConfigValidate",
		"config-validate",
		"config_validate",
		"Validate a config value without mutating runtime state.",
		CallKind::Helper,
		CallRoute::Client,
		false,
		Some(call_type(
			"crate::ConfigAssignment",
			&[named_type("ConfigAssignment", "crate::ConfigAssignment")],
		)),
		call_type("crate::OkView", &[named_type("OkView", "crate::OkView")]),
	),
	call(
		"EventsSubscribe",
		"events-subscribe",
		"events_subscribe",
		"Subscribe to runtime change events.",
		CallKind::Helper,
		CallRoute::Runtime,
		false,
		None,
		call_type(
			"crate::GlorpEventStreamView",
			&[named_type("GlorpEventStreamView", "crate::GlorpEventStreamView")],
		),
	),
	call(
		"EventsNext",
		"events-next",
		"events_next",
		"Poll the next event for a subscription token.",
		CallKind::Helper,
		CallRoute::Runtime,
		false,
		Some(call_type(
			"crate::StreamTokenInput",
			&[named_type("StreamTokenInput", "crate::StreamTokenInput")],
		)),
		call_type(
			"Option<crate::GlorpEvent>",
			&[named_type("GlorpEvent", "crate::GlorpEvent")],
		),
	),
	call(
		"EventsUnsubscribe",
		"events-unsubscribe",
		"events_unsubscribe",
		"Release a subscription token.",
		CallKind::Helper,
		CallRoute::Runtime,
		false,
		Some(call_type(
			"crate::StreamTokenInput",
			&[named_type("StreamTokenInput", "crate::StreamTokenInput")],
		)),
		call_type(
			"crate::TokenAckView",
			&[named_type("TokenAckView", "crate::TokenAckView")],
		),
	),
];
