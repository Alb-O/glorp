mod document;
mod editing;
mod geometry;
mod history;
mod layout;
mod layout_state;
mod navigation;
mod reducer;
mod selection;
mod session;
mod text;

#[cfg(test)]
mod tests;

use cosmic_text::{Buffer, FontSystem};
use iced::Point;

use std::fmt::{self, Display};
use std::ops::Range;
use std::sync::Arc;

use crate::overlay::{EditorOverlayTone, LayoutRect, OverlayPrimitive, OverlayRectKind};
use crate::scene::SceneConfig;

use self::document::DocumentState;
use self::geometry::{cluster_rectangle, selection_rectangles};
use self::history::{EditorSnapshot, HistoryEntry};
use self::layout::{BufferClusterInfo, BufferLayoutSnapshot};
use self::layout_state::EditorLayout;
use self::reducer::apply_intent;
use self::session::EditorSession;
use self::text::debug_snippet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EditorMode {
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

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EditorViewState {
	pub(crate) mode: EditorMode,
	pub(crate) selection: Option<Range<usize>>,
	pub(crate) selection_head: Option<usize>,
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

	fn range_cloned(&self) -> Range<usize> {
		self.range.clone()
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

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum EditorPointerIntent {
	BeginSelection { position: Point, select_word: bool },
	DragSelection(Point),
	EndSelection,
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
	pub(crate) document_changed: bool,
	pub(crate) view_changed: bool,
	pub(crate) selection_changed: bool,
	pub(crate) mode_changed: bool,
	pub(crate) requires_scene_rebuild: bool,
	pub(crate) viewport_target: Option<LayoutRect>,
	pub(crate) text_edit: Option<TextEdit>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ApplyResult {
	text_edit: Option<TextEdit>,
}

#[derive(Debug, Clone)]
pub(crate) struct EditorState {
	document: DocumentState,
	session: EditorSession,
}

#[derive(Debug, Clone)]
pub(crate) struct EditorLayoutModel {
	layout: EditorLayout,
}

#[derive(Debug, Clone)]
pub(crate) struct EditorEngine {
	state: EditorState,
	layout_model: EditorLayoutModel,
}

impl EditorOutcome {
	fn from_apply_result(previous_view: &EditorViewState, next_view: &EditorViewState, result: ApplyResult) -> Self {
		let document_changed = result.text_edit.is_some();
		Self {
			document_changed,
			view_changed: previous_view != next_view,
			selection_changed: previous_view.selection != next_view.selection
				|| previous_view.selection_head != next_view.selection_head,
			mode_changed: previous_view.mode != next_view.mode,
			requires_scene_rebuild: document_changed,
			viewport_target: next_view.viewport_target,
			text_edit: result.text_edit,
		}
	}
}

impl EditorViewState {
	pub(crate) fn overlay_count(&self, kind: OverlayRectKind) -> usize {
		self.overlays
			.iter()
			.filter(|primitive| {
				primitive
					.as_rect()
					.is_some_and(|(_, primitive_kind, _)| primitive_kind == kind)
			})
			.count()
	}
}

impl EditorState {
	fn new(text: impl Into<String>) -> Self {
		Self {
			document: DocumentState::new(text),
			session: EditorSession::new(),
		}
	}
}

impl EditorLayoutModel {
	fn new(font_system: &mut FontSystem, text: &str, config: SceneConfig) -> Self {
		Self {
			layout: EditorLayout::new(font_system, text, config),
		}
	}
}

impl EditorEngine {
	pub(crate) fn new(font_system: &mut FontSystem, text: impl Into<String>, config: SceneConfig) -> Self {
		let state = EditorState::new(text);
		let mut editor = Self {
			layout_model: EditorLayoutModel::new(font_system, state.document.text(), config),
			state,
		};
		editor.reset_normal_selection();
		editor.refresh_view_state();
		editor
	}

	pub(crate) fn text(&self) -> &str {
		self.state.document.text()
	}

	pub(crate) fn mode(&self) -> EditorMode {
		self.state.session.mode()
	}

	pub(crate) fn buffer(&self) -> Arc<Buffer> {
		self.layout_model.layout.buffer()
	}

	pub(crate) fn history_depths(&self) -> (usize, usize) {
		self.state.document.history_depths()
	}

	pub(crate) fn sync_buffer_config(&mut self, font_system: &mut FontSystem, config: SceneConfig) {
		let text = self.text().to_string();
		self.layout_model.layout.sync_buffer_config(font_system, &text, config);
		self.refresh_view_state();
	}

	pub(crate) fn sync_buffer_width(&mut self, font_system: &mut FontSystem, width: f32) {
		self.layout_model.layout.sync_buffer_width(font_system, width);
		self.refresh_view_state();
	}

	pub(crate) fn reset(&mut self, font_system: &mut FontSystem, text: impl Into<String>, config: SceneConfig) {
		self.state.document.reset(text);
		self.state.session = EditorSession::new();
		let text = self.text().to_string();
		self.layout_model.layout.reset(font_system, &text, config);
		self.reset_normal_selection();
		self.refresh_view_state();
	}

	pub(crate) fn view_state(&self) -> EditorViewState {
		self.layout_model.layout.view_state()
	}

	pub(crate) fn apply(&mut self, font_system: &mut FontSystem, intent: EditorIntent) -> EditorOutcome {
		let previous_view = self.layout_model.layout.view_state();
		let result = apply_intent(self, font_system, intent);
		self.refresh_view_state();
		EditorOutcome::from_apply_result(&previous_view, self.layout_model.layout.view_state_ref(), result)
	}

	pub(crate) fn selection_details(&self) -> String {
		let (undo_depth, redo_depth) = self.state.document.history_depths();
		match self.mode() {
			EditorMode::Normal => {
				let Some(selection) = self.selection() else {
					return format!("  mode: {}\n  selection: none", self.mode());
				};

				format!(
					"  mode: {}\n  bytes: {:?}\n  text: {}\n  rects: {}\n  active byte: {}\n  anchor byte: {}\n  undo/redo: {}/{}",
					self.mode(),
					selection.range(),
					self.preview_range(selection.range()),
					self.layout_model
						.layout
						.view_state_ref()
						.overlay_count(OverlayRectKind::EditorSelection(EditorOverlayTone::Normal)),
					self.caret(),
					self.pointer_anchor().unwrap_or(selection.range().start),
					undo_depth,
					redo_depth,
				)
			}
			EditorMode::Insert => self.selection().map_or_else(
				|| {
					format!(
						"  mode: {}\n  selection: none\n  undo/redo: {undo_depth}/{redo_depth}",
						self.mode()
					)
				},
				|selection| {
					format!(
						"  mode: {}\n  bytes: {:?}\n  text: {}\n  rects: {}\n  head byte: {}\n  undo/redo: {}/{}",
						self.mode(),
						selection.range(),
						self.preview_range(selection.range()),
						self.layout_model
							.layout
							.view_state_ref()
							.overlay_count(OverlayRectKind::EditorSelection(EditorOverlayTone::Insert)),
						selection.head(),
						undo_depth,
						redo_depth,
					)
				},
			),
		}
	}

	#[cfg(test)]
	pub(crate) fn buffer_text(&self) -> String {
		self.layout_model.layout.buffer_text()
	}

	fn layout_snapshot(&self) -> BufferLayoutSnapshot {
		self.layout_model.layout.snapshot(self.text())
	}

	fn reset_normal_selection(&mut self) {
		if let Some(selection) = self
			.layout_snapshot()
			.cluster(0)
			.map(|cluster| cluster.byte_range.clone())
		{
			let head = selection.start;
			self.state
				.session
				.set_normal_selection(EditorSelection::new(selection, head), None, Some(head));
		} else {
			self.set_selection(None);
			self.clear_pointer_anchor();
		}
	}

	fn select_cluster(&mut self, layout: &BufferLayoutSnapshot, cluster_index: usize) {
		let Some(cluster) = layout.cluster(cluster_index) else {
			return;
		};

		self.state.session.set_normal_selection(
			EditorSelection::new(cluster.byte_range.clone(), cluster.byte_range.start),
			Some(cluster.center_x()),
			Some(cluster.byte_range.start),
		);
	}

	fn active_selection_index(&self, layout: &BufferLayoutSnapshot) -> Option<usize> {
		if self.selection().is_none() {
			return None;
		}

		layout
			.cluster_at_or_after(self.caret())
			.or_else(|| layout.cluster_before(self.caret().saturating_add(1)))
	}

	fn active_selection<'a>(&self, layout: &'a BufferLayoutSnapshot) -> Option<&'a BufferClusterInfo> {
		self.active_selection_index(layout)
			.and_then(|index| layout.cluster(index))
	}

	fn preview_range(&self, range: &Range<usize>) -> String {
		self.text()
			.get(range.clone())
			.map(debug_snippet)
			.unwrap_or_else(|| "<invalid utf8 slice>".to_string())
	}

	fn history_snapshot(&self) -> EditorSnapshot {
		self.state.session.history_snapshot()
	}

	fn restore_snapshot(&mut self, snapshot: &EditorSnapshot) {
		self.state.session.restore_snapshot(snapshot, self.state.document.len());
	}

	fn apply_document_edit(&mut self, font_system: &mut FontSystem, edit: &TextEdit) -> TextEdit {
		self.apply_buffer_edit(font_system, edit);
		self.state.document.apply_edit(edit)
	}

	fn record_history(&mut self, forward: TextEdit, inverse: TextEdit, before: EditorSnapshot) {
		self.state.document.record_history(HistoryEntry {
			forward,
			inverse,
			before,
			after: self.history_snapshot(),
		});
	}

	fn active_viewport_target(&self, layout: &BufferLayoutSnapshot) -> Option<LayoutRect> {
		if matches!(self.mode(), EditorMode::Insert) {
			return self.layout_model.layout.insert_cursor_block(self.text(), self.caret());
		}

		self.active_selection(layout).map(cluster_rectangle)
	}

	fn refresh_view_state(&mut self) {
		let layout = self.layout_snapshot();
		let selection = self.selection().cloned();
		let selection_head = selection.as_ref().map(EditorSelection::head);
		let tone = EditorOverlayTone::from(self.mode());
		let insert_cursor = matches!(self.mode(), EditorMode::Insert)
			.then(|| {
				selection_head.and_then(|head| self.layout_model.layout.insert_cursor_rectangle(self.text(), head))
			})
			.flatten();
		let viewport_target = self.active_viewport_target(&layout);
		self.layout_model.layout.set_view_state(EditorViewState {
			mode: self.mode(),
			selection: selection.as_ref().map(EditorSelection::range_cloned),
			selection_head,
			overlays: self.build_overlays(&layout, selection.as_ref(), insert_cursor, viewport_target, tone),
			viewport_target,
		});
	}

	fn build_overlays(
		&self, layout: &BufferLayoutSnapshot, selection: Option<&EditorSelection>, insert_cursor: Option<LayoutRect>,
		viewport_target: Option<LayoutRect>, tone: EditorOverlayTone,
	) -> Arc<[OverlayPrimitive]> {
		let mut overlays = Vec::new();

		if matches!(self.mode(), EditorMode::Insert) {
			if let Some(insert_block) = viewport_target {
				overlays.push(OverlayPrimitive::scene_rect(
					insert_block,
					OverlayRectKind::EditorInsertBlock(tone),
				));
			}

			if let Some(caret) = insert_cursor {
				overlays.push(OverlayPrimitive::scene_rect(caret, OverlayRectKind::EditorCaret(tone)));
			}
		} else {
			if let Some(selection) = selection {
				overlays.extend(
					selection_rectangles(layout, selection.range())
						.iter()
						.copied()
						.map(|rect| OverlayPrimitive::scene_rect(rect, OverlayRectKind::EditorSelection(tone))),
				);
			}

			if let Some(active) = viewport_target {
				overlays.push(OverlayPrimitive::scene_rect(
					active,
					OverlayRectKind::EditorActive(tone),
				));
			}
		}

		overlays.into()
	}

	fn selection(&self) -> Option<&EditorSelection> {
		self.state.session.selection()
	}

	fn selection_range(&self) -> Option<Range<usize>> {
		self.selection().map(EditorSelection::range_cloned)
	}

	fn set_selection(&mut self, selection: Option<EditorSelection>) {
		self.state.session.set_selection(selection);
	}

	fn set_mode(&mut self, mode: EditorMode) {
		self.state.session.set_mode(mode);
	}

	fn caret(&self) -> usize {
		self.state.session.caret()
	}

	fn preferred_x(&self) -> Option<f32> {
		self.state.session.preferred_x()
	}

	fn set_preferred_x(&mut self, preferred_x: Option<f32>) {
		self.state.session.set_preferred_x(preferred_x);
	}

	fn pointer_anchor(&self) -> Option<usize> {
		self.state.session.pointer_anchor()
	}

	fn set_pointer_anchor(&mut self, pointer_anchor: Option<usize>) {
		self.state.session.set_pointer_anchor(pointer_anchor);
	}

	fn clear_pointer_anchor(&mut self) {
		self.set_pointer_anchor(None);
	}

	fn enter_insert_at(&mut self, caret: usize) {
		let layout = self.layout_snapshot();
		self.set_insert_head(&layout, caret);
	}

	fn buffer_hit(&self, point: Point) -> Option<cosmic_text::Cursor> {
		self.layout_model.layout.hit(point)
	}

	fn apply_buffer_edit(&mut self, font_system: &mut FontSystem, edit: &TextEdit) {
		let text = self.text().to_string();
		self.layout_model.layout.apply_edit(font_system, &text, edit);
	}

	fn insert_selection(&self, layout: &BufferLayoutSnapshot, head: usize) -> Option<EditorSelection> {
		layout
			.cluster_at_insert_head(head)
			.and_then(|index| layout.cluster(index))
			.map(|cluster| EditorSelection::new(cluster.byte_range.clone(), head))
	}

	fn set_insert_head(&mut self, layout: &BufferLayoutSnapshot, head: usize) {
		self.state.session.enter_insert(self.insert_selection(layout, head));
	}
}
