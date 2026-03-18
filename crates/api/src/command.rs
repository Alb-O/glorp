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
	EditorPointerBegin(EditorPointerBeginInput),
	EditorPointerDrag(EditorPointerDragInput),
	EditorPointerEnd,
	UiSidebarSelect(SidebarTabInput),
	UiInspectTargetHover(InspectTargetInput),
	UiInspectTargetSelect(InspectTargetInput),
	UiCanvasFocusSet(CanvasFocusInput),
	UiViewportScrollTo(ScrollTarget),
	UiViewportMetricsSet(ViewportMetricsInput),
	UiPaneRatioSet(PaneRatioInput),
	SceneEnsure,
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct SidebarTabInput {
	pub tab: SidebarTab,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct ScrollTarget {
	pub x: f32,
	pub y: f32,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct PaneRatioInput {
	pub ratio: f32,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct ViewportMetricsInput {
	pub layout_width: f32,
	pub viewport_width: f32,
	pub viewport_height: f32,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct CanvasFocusInput {
	pub focused: bool,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct InspectTargetInput {
	pub target: Option<CanvasTarget>,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct EditorPointerBeginInput {
	pub x: f32,
	pub y: f32,
	pub select_word: bool,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct EditorPointerDragInput {
	pub x: f32,
	pub y: f32,
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

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum SidebarTab {
	Controls,
	Inspect,
	Perf,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash)]
#[serde(tag = "kind", content = "value", rename_all = "kebab-case")]
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
