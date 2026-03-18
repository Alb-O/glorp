use crate::{ConfigAssignment, ConfigPath, GlorpTxn, GlorpValue};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum GlorpCommand {
	Txn(GlorpTxn),
	Config(ConfigCommand),
	Document(DocumentCommand),
	Editor(EditorCommand),
	Ui(UiCommand),
	Scene(SceneCommand),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum ConfigCommand {
	Set { path: ConfigPath, value: GlorpValue },
	Patch { values: Vec<ConfigAssignment> },
	Reset { path: ConfigPath },
	Reload,
	Persist,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum DocumentCommand {
	Replace { text: String },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum EditorCommand {
	Motion(EditorMotion),
	Mode(EditorModeCommand),
	Edit(EditorEditCommand),
	History(EditorHistoryCommand),
	Pointer(EditorPointerCommand),
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum EditorEditCommand {
	Backspace,
	DeleteForward,
	DeleteSelection,
	Insert { text: String },
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum EditorHistoryCommand {
	Undo,
	Redo,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum EditorPointerCommand {
	Begin { x: f32, y: f32, select_word: bool },
	Drag { x: f32, y: f32 },
	End,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum UiCommand {
	SidebarSelect { tab: SidebarTab },
	InspectTargetSelect { target: Option<CanvasTarget> },
	ViewportScrollTo { x: f32, y: f32 },
	PaneRatioSet { ratio: f32 },
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum SceneCommand {
	Ensure,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum SidebarTab {
	Controls,
	Inspect,
	Perf,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash)]
pub enum CanvasTarget {
	Run(usize),
	Cluster(usize),
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

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq)]
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
