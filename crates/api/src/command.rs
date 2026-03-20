use crate::ConfigPath;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct GlorpCall {
	pub id: String,
	pub input: Option<crate::GlorpValue>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct GlorpCallResult {
	pub id: String,
	pub output: crate::GlorpValue,
}

impl GlorpCall {
	#[must_use]
	pub fn new(id: impl Into<String>, input: impl Into<Option<crate::GlorpValue>>) -> Self {
		Self {
			id: id.into(),
			input: input.into(),
		}
	}
}

impl GlorpCallResult {
	#[must_use]
	pub fn new(id: impl Into<String>, output: impl Into<crate::GlorpValue>) -> Self {
		Self {
			id: id.into(),
			output: output.into(),
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
pub struct EditorHistoryInput {
	pub action: EditorHistoryCommand,
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
