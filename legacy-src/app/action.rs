use {
	super::{
		session::{SceneDemand, SessionRequest},
		state::EditorDispatchSource,
	},
	crate::{
		editor::{EditorIntent, EditorPointerIntent},
		types::{
			CanvasEvent, CanvasTarget, ControlsMessage, Message, PerfMessage, ShellMessage, SidebarMessage, SidebarTab,
			ViewportMessage,
		},
	},
	iced::{Size, Vector, widget::pane_grid},
};

#[derive(Debug, Clone)]
pub(super) enum AppAction {
	Control(ControlsMessage),
	ReplaceDocument(String),
	SelectSidebarTab(SidebarTab),
	HoverCanvas(Option<CanvasTarget>),
	SetCanvasFocus(bool),
	SetCanvasScroll(Vector),
	BeginPointerSelection {
		target: Option<CanvasTarget>,
		intent: EditorPointerIntent,
	},
	Editor {
		intent: EditorIntent,
		source: EditorDispatchSource,
	},
	PerfTick,
	ObserveCanvasResize(Size),
	FlushResizeReflow,
	ResizePane(pane_grid::ResizeEvent),
	EnsureScene,
}

#[derive(Debug, Clone)]
pub(super) struct SessionEffect {
	pub(super) request: SessionRequest,
	pub(super) demand: SceneDemand,
	pub(super) policy: SessionUiPolicy,
}

impl AppAction {
	pub(super) fn editor(intent: EditorIntent) -> Self {
		Self::Editor {
			source: editor_dispatch_source(&intent),
			intent,
		}
	}

	pub(super) fn is_editor_command(&self) -> bool {
		matches!(self, Self::BeginPointerSelection { .. } | Self::Editor { .. })
	}
}

impl From<Message> for AppAction {
	fn from(message: Message) -> Self {
		match message {
			Message::Controls(message) => Self::Control(message),
			Message::Sidebar(SidebarMessage::SelectTab(tab)) => Self::SelectSidebarTab(tab),
			Message::Canvas(CanvasEvent::Hovered(target)) => Self::HoverCanvas(target),
			Message::Canvas(CanvasEvent::FocusChanged(focused)) => Self::SetCanvasFocus(focused),
			Message::Canvas(CanvasEvent::ScrollChanged(scroll)) => Self::SetCanvasScroll(scroll),
			Message::Canvas(CanvasEvent::PointerSelectionStarted { target, intent }) => {
				Self::BeginPointerSelection { target, intent }
			}
			Message::Editor(intent) => Self::editor(intent),
			Message::Perf(PerfMessage::Tick(_now)) => Self::PerfTick,
			Message::Viewport(ViewportMessage::CanvasResized(size)) => Self::ObserveCanvasResize(size),
			Message::Viewport(ViewportMessage::ResizeTick(_now)) => Self::FlushResizeReflow,
			Message::Shell(ShellMessage::PaneResized(event)) => Self::ResizePane(event),
		}
	}
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum ScrollBehavior {
	#[default]
	KeepClamped,
	ResetScroll,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct SessionUiPolicy {
	pub(super) scroll_behavior: ScrollBehavior,
	pub(super) reveal_viewport: bool,
	pub(super) scene_refresh_reason: Option<SceneRefreshReason>,
}

impl SessionUiPolicy {
	pub(super) fn keep() -> Self {
		Self::default()
	}

	pub(super) fn reset_scroll(reason: SceneRefreshReason) -> Self {
		Self {
			scroll_behavior: ScrollBehavior::ResetScroll,
			scene_refresh_reason: Some(reason),
			..Self::default()
		}
	}

	pub(super) fn scene_refresh(reason: SceneRefreshReason) -> Self {
		Self {
			scene_refresh_reason: Some(reason),
			..Self::default()
		}
	}

	pub(super) fn reveal(reveal_viewport: bool, scene_refresh_reason: Option<SceneRefreshReason>) -> Self {
		Self {
			reveal_viewport,
			scene_refresh_reason,
			..Self::default()
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SceneRefreshReason {
	PresetLoaded,
	ControlsChanged,
	TextEdited,
	ResizeReflow,
}

impl SceneRefreshReason {
	pub(super) fn records_resize_reflow(self) -> bool {
		matches!(self, Self::ResizeReflow)
	}
}

fn editor_dispatch_source(intent: &EditorIntent) -> EditorDispatchSource {
	match intent {
		EditorIntent::Pointer(EditorPointerIntent::Begin { .. }) => EditorDispatchSource::PointerPress,
		EditorIntent::Pointer(EditorPointerIntent::Drag(_)) => EditorDispatchSource::PointerDrag,
		EditorIntent::Pointer(EditorPointerIntent::End) => EditorDispatchSource::PointerRelease,
		_ => EditorDispatchSource::Keyboard,
	}
}
