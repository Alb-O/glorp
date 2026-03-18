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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub(crate) enum EditorMode {
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

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct EditorViewState {
	pub(crate) mode: EditorMode,
	pub(crate) selection: Option<Range<usize>>,
	pub(crate) selection_head: Option<usize>,
	pub(crate) pointer_anchor: Option<usize>,
	pub(crate) overlays: Arc<[OverlayPrimitive]>,
	pub(crate) viewport_target: Option<LayoutRect>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct EditorSelection {
	range: Range<usize>,
	head: usize,
}

impl EditorSelection {
	fn new(range: Range<usize>, head: usize) -> Self {
		Self { range, head }
	}

	fn clamped(&self, document_len: usize) -> Self {
		let start = self.range.start.min(document_len);
		let end = self.range.end.min(document_len).max(start);
		let head = self.head.min(document_len);
		Self {
			range: start..end,
			head,
		}
	}

	fn range(&self) -> &Range<usize> {
		&self.range
	}

	fn head(&self) -> usize {
		self.head
	}
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum EditorIntent {
	Pointer(EditorPointerIntent),
	Motion(EditorMotion),
	Mode(EditorModeIntent),
	Edit(EditorEditIntent),
	History(EditorHistoryIntent),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum EditorPointerIntent {
	Begin { position: Point, select_word: bool },
	Drag(Point),
	End,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EditorMotion {
	Left,
	Right,
	Up,
	Down,
	LineStart,
	LineEnd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EditorModeIntent {
	EnterInsertBefore,
	EnterInsertAfter,
	ExitInsert,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum EditorEditIntent {
	Backspace,
	DeleteForward,
	DeleteSelection,
	InsertText(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EditorHistoryIntent {
	Undo,
	Redo,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TextEdit {
	pub(crate) range: Range<usize>,
	pub(crate) inserted: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct EditorOutcome {
	pub(crate) view_changed: bool,
	pub(crate) selection_changed: bool,
	pub(crate) mode_changed: bool,
	pub(crate) viewport_target: Option<LayoutRect>,
	pub(crate) text_edit: Option<TextEdit>,
}

#[derive(Debug, Clone, Default)]
struct ApplyResult {
	text_edit: Option<TextEdit>,
	layout: Option<DocumentLayout>,
	view_refreshed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct EditorViewportMetrics {
	pub(crate) wrapping: WrapChoice,
	pub(crate) measured_width: f32,
	pub(crate) measured_height: f32,
}

#[derive(Debug, Clone)]
pub(crate) struct EditorTextLayerState {
	pub(crate) buffer: std::sync::Weak<cosmic_text::Buffer>,
	pub(crate) measured_height: f32,
}

#[derive(Debug, Clone)]
pub(crate) struct EditorEngine {
	core: EditorCore,
	layout: EditorLayout,
}

impl EditorOutcome {
	fn from_apply_result(previous_view: &EditorViewState, next_view: &EditorViewState, result: ApplyResult) -> Self {
		Self {
			view_changed: previous_view != next_view,
			selection_changed: previous_view.selection != next_view.selection
				|| previous_view.selection_head != next_view.selection_head,
			mode_changed: previous_view.mode != next_view.mode,
			viewport_target: next_view.viewport_target,
			text_edit: result.text_edit,
		}
	}

	#[cfg(test)]
	pub(crate) fn document_changed(&self) -> bool {
		self.text_edit.is_some()
	}

	#[cfg(test)]
	pub(crate) fn requires_scene_rebuild(&self) -> bool {
		self.document_changed()
	}
}

impl EditorViewState {
	pub(crate) fn overlay_count(&self, kind: OverlayRectKind) -> usize {
		self.overlays.iter().filter(|primitive| primitive.kind == kind).count()
	}
}

impl EditorEngine {
	pub(crate) fn new(font_system: &mut FontSystem, text: impl Into<String>, config: SceneConfig) -> Self {
		let text = text.into();
		let mut editor = Self {
			core: EditorCore::new(text.clone()),
			layout: EditorLayout::new(font_system, &text, config),
		};
		editor.reset_normal_selection();
		editor.refresh_view_state(None);
		editor
	}

	pub(crate) fn apply(&mut self, font_system: &mut FontSystem, intent: EditorIntent) -> EditorOutcome {
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
		EditorOutcome::from_apply_result(
			&previous_view,
			self.layout.view_state_ref(),
			ApplyResult {
				text_edit,
				layout: None,
				view_refreshed: false,
			},
		)
	}
}
