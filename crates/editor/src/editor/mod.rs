mod core;
mod document;
mod editing;
mod geometry;
mod history;
mod layout_state;
mod navigation;
mod projection;
mod reducer;
mod selection;
mod session;
mod text;

#[cfg(test)]
mod tests;

use {
	self::{core::EditorCore, layout_state::EditorLayout, reducer::apply_intent},
	crate::{
		overlay::{LayoutRect, OverlayPrimitive, OverlayRectKind},
		scene::{DocumentLayout, SceneConfig},
		telemetry::duration_ms,
		types::WrapChoice,
	},
	cosmic_text::FontSystem,
	iced::Point,
	std::{
		fmt::{self, Display},
		ops::Range,
		sync::Arc,
		time::Instant,
	},
	tracing::{debug, trace, trace_span, warn},
};

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, Default, PartialEq, Eq, Hash)]
pub enum EditorMode {
	#[default]
	Normal,
	Insert,
}

impl Display for EditorMode {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Normal => f.write_str("Normal"),
			Self::Insert => f.write_str("Insert"),
		}
	}
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, PartialEq)]
pub struct EditorViewState {
	pub mode: EditorMode,
	pub selection: Option<Range<usize>>,
	pub selection_head: Option<usize>,
	pub pointer_anchor: Option<usize>,
	pub overlays: Arc<[OverlayPrimitive]>,
	pub viewport_target: Option<LayoutRect>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorSelection {
	range: Range<usize>,
	head: usize,
}

impl EditorSelection {
	const fn new(range: Range<usize>, head: usize) -> Self {
		Self { range, head }
	}

	const fn range(&self) -> &Range<usize> {
		&self.range
	}

	const fn head(&self) -> usize {
		self.head
	}
}

#[derive(Debug, Clone, PartialEq)]
pub enum EditorIntent {
	Pointer(EditorPointerIntent),
	Motion(EditorMotion),
	Mode(EditorModeIntent),
	Edit(EditorEditIntent),
	History(EditorHistoryIntent),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EditorPointerIntent {
	Begin { position: Point, select_word: bool },
	Drag(Point),
	End,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorMotion {
	Left,
	Right,
	Up,
	Down,
	LineStart,
	LineEnd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorModeIntent {
	EnterInsertBefore,
	EnterInsertAfter,
	ExitInsert,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorEditIntent {
	Backspace,
	DeleteForward,
	DeleteSelection,
	InsertText(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorHistoryIntent {
	Undo,
	Redo,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEdit {
	pub range: Range<usize>,
	pub inserted: String,
}

impl TextEdit {
	fn insert(at: usize, inserted: String) -> Self {
		Self {
			range: at..at,
			inserted,
		}
	}

	fn delete(range: Range<usize>) -> Self {
		Self {
			range,
			inserted: String::new(),
		}
	}
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct EditorOutcome {
	pub view_changed: bool,
	pub selection_changed: bool,
	pub mode_changed: bool,
	pub viewport_target: Option<LayoutRect>,
	pub text_edit: Option<TextEdit>,
}

#[derive(Debug, Clone, Default)]
pub struct ApplyResult {
	text_edit: Option<TextEdit>,
	layout: Option<DocumentLayout>,
	view_refreshed: bool,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct EditorViewportMetrics {
	pub wrapping: WrapChoice,
	pub measured_width: f32,
	pub measured_height: f32,
}

#[derive(Debug, Clone)]
pub struct EditorTextLayerState {
	pub buffer: std::sync::Weak<cosmic_text::Buffer>,
	pub measured_height: f32,
}

#[derive(Debug, Clone)]
pub struct EditorEngine {
	core: EditorCore,
	layout: EditorLayout,
}

impl EditorOutcome {
	fn from_views(previous_view: &EditorViewState, next_view: &EditorViewState, text_edit: Option<TextEdit>) -> Self {
		Self {
			view_changed: previous_view != next_view,
			selection_changed: previous_view.selection != next_view.selection
				|| previous_view.selection_head != next_view.selection_head,
			mode_changed: previous_view.mode != next_view.mode,
			viewport_target: next_view.viewport_target,
			text_edit,
		}
	}

	#[cfg(test)]
	#[must_use]
	pub const fn document_changed(&self) -> bool {
		self.text_edit.is_some()
	}

	#[cfg(test)]
	#[must_use]
	pub const fn requires_scene_rebuild(&self) -> bool {
		self.document_changed()
	}
}

impl EditorViewState {
	#[must_use]
	pub fn overlay_count(&self, kind: OverlayRectKind) -> usize {
		self.overlays.iter().filter(|primitive| primitive.kind == kind).count()
	}
}

impl EditorEngine {
	pub fn new(font_system: &mut FontSystem, text: impl Into<String>, config: SceneConfig) -> Self {
		let text = text.into();
		let layout = EditorLayout::new(font_system, &text, config);
		let mut editor = Self {
			core: EditorCore::new(text),
			layout,
		};
		editor.reset_normal_selection();
		editor.refresh_view_state(None);
		editor
	}

	pub fn apply(&mut self, font_system: &mut FontSystem, intent: EditorIntent) -> EditorOutcome {
		let _span = trace_span!("editor.apply", intent = ?intent).entered();
		let previous_view = self.view_state();
		let apply_started = Instant::now();
		let ApplyResult {
			text_edit,
			layout,
			view_refreshed,
		} = apply_intent(self, font_system, intent);
		let reducer_elapsed = apply_started.elapsed();
		let refresh_started = Instant::now();
		if !view_refreshed {
			self.refresh_view_state(layout);
		}
		let refresh_elapsed = refresh_started.elapsed();
		let total_elapsed = apply_started.elapsed();
		let total_ms = duration_ms(total_elapsed);
		if total_ms >= 16.7 {
			warn!(
				reducer_ms = duration_ms(reducer_elapsed),
				refresh_ms = duration_ms(refresh_elapsed),
				total_ms,
				text_bytes = self.text().len(),
				"editor apply over frame budget"
			);
		} else if total_ms >= 8.0 {
			debug!(
				reducer_ms = duration_ms(reducer_elapsed),
				refresh_ms = duration_ms(refresh_elapsed),
				total_ms,
				text_bytes = self.text().len(),
				"editor apply over warning threshold"
			);
		} else {
			trace!(
				reducer_ms = duration_ms(reducer_elapsed),
				refresh_ms = duration_ms(refresh_elapsed),
				total_ms,
				text_bytes = self.text().len(),
				"editor apply"
			);
		}
		EditorOutcome::from_views(&previous_view, self.layout.view_state_ref(), text_edit)
	}
}
