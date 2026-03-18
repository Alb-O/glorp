use crate::{ConfigAssignment, ConfigPath, GlorpTxn, GlorpValue};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(tag = "op", content = "input", rename_all = "kebab-case")]
pub enum GlorpExec {
	Txn(GlorpTxn),
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
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct ConfigPatchInput {
	pub patch: GlorpValue,
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
#[serde(rename_all = "kebab-case")]
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
