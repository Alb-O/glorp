use {
	crate::state::UiRuntimeState,
	glorp_api::{GlorpConfig, GlorpRevisions},
	glorp_editor::{CanvasTarget, SessionSnapshot},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SidebarTab {
	Controls,
	Inspect,
	Perf,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GuiCommand {
	SidebarSelect(SidebarTab),
	InspectTargetHover(Option<CanvasTarget>),
	InspectTargetSelect(Option<CanvasTarget>),
	CanvasFocusSet(bool),
	ViewportScrollTo {
		x: f32,
		y: f32,
	},
	ViewportMetricsSet {
		layout_width: f32,
		viewport_width: f32,
		viewport_height: f32,
	},
	PaneRatioSet(f32),
	ShowBaselinesSet(bool),
	ShowHitboxesSet(bool),
	EditorPointerBegin {
		x: f32,
		y: f32,
		select_word: bool,
	},
	EditorPointerDrag {
		x: f32,
		y: f32,
	},
	EditorPointerEnd,
	SceneEnsure,
}

#[derive(Debug, Clone)]
pub struct GuiRuntimeFrame {
	pub config: GlorpConfig,
	pub ui: UiRuntimeState,
	pub revisions: GlorpRevisions,
	pub snapshot: SessionSnapshot,
	pub document_text: String,
}
