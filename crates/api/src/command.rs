use crate::{ConfigAssignment, ConfigPath};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(tag = "op", content = "input", rename_all = "kebab-case")]
pub enum GlorpCall {
	Txn(crate::GlorpTxn),
	ConfigSet(ConfigAssignment),
	ConfigReset(ConfigPathInput),
	ConfigPatch(ConfigPatchInput),
	ConfigReload,
	ConfigPersist,
	DocumentReplace(TextInput),
	EditorMotion(EditorMotionInput),
	EditorMode(EditorModeInput),
	EditorInsert(TextInput),
	EditorBackspace,
	EditorDeleteForward,
	EditorDeleteSelection,
	EditorHistory(EditorHistoryInput),
	Schema,
	Config,
	DocumentText,
	Editor,
	Capabilities,
	SessionAttach,
	SessionShutdown,
	ConfigValidate(ConfigAssignment),
	EventsSubscribe,
	EventsNext(crate::StreamTokenInput),
	EventsUnsubscribe(crate::StreamTokenInput),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(tag = "op", content = "output", rename_all = "kebab-case")]
pub enum GlorpCallResult {
	Txn(crate::GlorpOutcome),
	ConfigSet(crate::GlorpOutcome),
	ConfigReset(crate::GlorpOutcome),
	ConfigPatch(crate::GlorpOutcome),
	ConfigReload(crate::GlorpOutcome),
	ConfigPersist(crate::GlorpOutcome),
	DocumentReplace(crate::GlorpOutcome),
	EditorMotion(crate::GlorpOutcome),
	EditorMode(crate::GlorpOutcome),
	EditorInsert(crate::GlorpOutcome),
	EditorBackspace(crate::GlorpOutcome),
	EditorDeleteForward(crate::GlorpOutcome),
	EditorDeleteSelection(crate::GlorpOutcome),
	EditorHistory(crate::GlorpOutcome),
	Schema(crate::GlorpSchema),
	Config(crate::GlorpConfig),
	DocumentText(String),
	Editor(crate::EditorStateView),
	Capabilities(crate::GlorpCapabilities),
	SessionAttach(crate::GlorpSessionView),
	SessionShutdown(crate::OkView),
	ConfigValidate(crate::OkView),
	EventsSubscribe(crate::GlorpEventStreamView),
	EventsNext(Option<crate::GlorpEvent>),
	EventsUnsubscribe(crate::TokenAckView),
}

impl GlorpCall {
	pub fn id(&self) -> &'static str {
		match self {
			Self::Txn(_) => "txn",
			Self::ConfigSet(_) => "config-set",
			Self::ConfigReset(_) => "config-reset",
			Self::ConfigPatch(_) => "config-patch",
			Self::ConfigReload => "config-reload",
			Self::ConfigPersist => "config-persist",
			Self::DocumentReplace(_) => "document-replace",
			Self::EditorMotion(_) => "editor-motion",
			Self::EditorMode(_) => "editor-mode",
			Self::EditorInsert(_) => "editor-insert",
			Self::EditorBackspace => "editor-backspace",
			Self::EditorDeleteForward => "editor-delete-forward",
			Self::EditorDeleteSelection => "editor-delete-selection",
			Self::EditorHistory(_) => "editor-history",
			Self::Schema => "schema",
			Self::Config => "config",
			Self::DocumentText => "document-text",
			Self::Editor => "editor",
			Self::Capabilities => "capabilities",
			Self::SessionAttach => "session-attach",
			Self::SessionShutdown => "session-shutdown",
			Self::ConfigValidate(_) => "config-validate",
			Self::EventsSubscribe => "events-subscribe",
			Self::EventsNext(_) => "events-next",
			Self::EventsUnsubscribe(_) => "events-unsubscribe",
		}
	}
}

impl GlorpCallResult {
	pub fn id(&self) -> &'static str {
		match self {
			Self::Txn(_) => "txn",
			Self::ConfigSet(_) => "config-set",
			Self::ConfigReset(_) => "config-reset",
			Self::ConfigPatch(_) => "config-patch",
			Self::ConfigReload(_) => "config-reload",
			Self::ConfigPersist(_) => "config-persist",
			Self::DocumentReplace(_) => "document-replace",
			Self::EditorMotion(_) => "editor-motion",
			Self::EditorMode(_) => "editor-mode",
			Self::EditorInsert(_) => "editor-insert",
			Self::EditorBackspace(_) => "editor-backspace",
			Self::EditorDeleteForward(_) => "editor-delete-forward",
			Self::EditorDeleteSelection(_) => "editor-delete-selection",
			Self::EditorHistory(_) => "editor-history",
			Self::Schema(_) => "schema",
			Self::Config(_) => "config",
			Self::DocumentText(_) => "document-text",
			Self::Editor(_) => "editor",
			Self::Capabilities(_) => "capabilities",
			Self::SessionAttach(_) => "session-attach",
			Self::SessionShutdown(_) => "session-shutdown",
			Self::ConfigValidate(_) => "config-validate",
			Self::EventsSubscribe(_) => "events-subscribe",
			Self::EventsNext(_) => "events-next",
			Self::EventsUnsubscribe(_) => "events-unsubscribe",
		}
	}

	pub fn into_output_value(self) -> serde_json::Result<serde_json::Value> {
		match serde_json::to_value(self)? {
			serde_json::Value::Object(mut object) => Ok(object.remove("output").unwrap_or(serde_json::Value::Null)),
			other => Ok(other),
		}
	}

	pub fn into_outcome(self) -> Result<crate::GlorpOutcome, Self> {
		match self {
			Self::Txn(outcome)
			| Self::ConfigSet(outcome)
			| Self::ConfigReset(outcome)
			| Self::ConfigPatch(outcome)
			| Self::ConfigReload(outcome)
			| Self::ConfigPersist(outcome)
			| Self::DocumentReplace(outcome)
			| Self::EditorMotion(outcome)
			| Self::EditorMode(outcome)
			| Self::EditorInsert(outcome)
			| Self::EditorBackspace(outcome)
			| Self::EditorDeleteForward(outcome)
			| Self::EditorDeleteSelection(outcome)
			| Self::EditorHistory(outcome) => Ok(outcome),
			other => Err(other),
		}
	}
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct ConfigPatchInput {
	pub patch: crate::GlorpValue,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct ConfigPathInput {
	pub path: ConfigPath,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct TextInput {
	pub text: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct EditorMotionInput {
	pub motion: EditorMotion,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct EditorModeInput {
	pub mode: EditorModeCommand,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct EditorHistoryInput {
	pub action: EditorHistoryCommand,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum EditorMotion {
	Left,
	Right,
	Up,
	Down,
	LineStart,
	LineEnd,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum EditorModeCommand {
	EnterInsertBefore,
	EnterInsertAfter,
	ExitInsert,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum EditorHistoryCommand {
	Undo,
	Redo,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum EditorMode {
	Normal,
	Insert,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum SamplePreset {
	Tall,
	Mixed,
	Rust,
	Ligatures,
	Arabic,
	Cjk,
	Emoji,
	Custom,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum FontChoice {
	#[serde(rename = "jetbrains-mono")]
	JetBrainsMono,
	#[serde(rename = "monospace")]
	Monospace,
	#[serde(rename = "noto-sans-cjk")]
	NotoSansCjk,
	#[serde(rename = "sans-serif")]
	SansSerif,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ShapingChoice {
	Auto,
	Basic,
	Advanced,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum WrapChoice {
	None,
	Word,
	Glyph,
	WordOrGlyph,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct TextRange {
	pub start: u64,
	pub end: u64,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct LayoutRectView {
	pub x: f32,
	pub y: f32,
	pub width: f32,
	pub height: f32,
}
