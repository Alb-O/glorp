mod document;
mod editing;
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

use crate::scene::SceneConfig;

use self::document::DocumentState;
use self::history::{EditorSnapshot, HistoryEntry};
use self::layout::{BufferClusterInfo, BufferLayoutSnapshot};
use self::layout_state::EditorLayout;
use self::reducer::apply_command;
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
	#[cfg(test)]
	pub(crate) selection: Option<Range<usize>>,
	#[cfg(test)]
	pub(crate) selection_head: Option<usize>,
	pub(crate) selection_rectangles: Arc<[EditorSelectionRect]>,
	pub(crate) viewport_target: Option<EditorSelectionRect>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct EditorSelectionRect {
	pub(crate) x: f32,
	pub(crate) y: f32,
	pub(crate) width: f32,
	pub(crate) height: f32,
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

#[derive(Debug, Clone)]
pub(crate) struct EditorBuffer {
	document: DocumentState,
	session: EditorSession,
	layout: EditorLayout,
}

#[derive(Debug, Clone)]
pub(crate) enum EditorCommand {
	BeginPointerSelection { position: Point, select_word: bool },
	DragPointerSelection(Point),
	EndPointerSelection,
	MoveLeft,
	MoveRight,
	MoveUp,
	MoveDown,
	MoveLineStart,
	MoveLineEnd,
	EnterInsertBefore,
	EnterInsertAfter,
	ExitInsert,
	Undo,
	Redo,
	Backspace,
	DeleteForward,
	DeleteSelection,
	InsertText(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TextEdit {
	pub(crate) range: Range<usize>,
	pub(crate) inserted: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum EditorEffect {
	DocumentChanged(TextEdit),
	ViewChanged,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct EditorUpdate {
	effects: Vec<EditorEffect>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ApplyResult {
	text_edit: Option<TextEdit>,
}

impl EditorUpdate {
	fn from_apply_result(previous_view: &EditorViewState, next_view: &EditorViewState, result: ApplyResult) -> Self {
		let mut effects = Vec::new();

		if let Some(text_edit) = result.text_edit {
			effects.push(EditorEffect::DocumentChanged(text_edit));
		}

		if previous_view != next_view {
			effects.push(EditorEffect::ViewChanged);
		}

		Self { effects }
	}

	pub(crate) fn document_changed(&self) -> bool {
		self.effects
			.iter()
			.any(|effect| matches!(effect, EditorEffect::DocumentChanged(_)))
	}

	pub(crate) fn view_changed(&self) -> bool {
		self.effects.contains(&EditorEffect::ViewChanged)
	}

	#[cfg(test)]
	pub(crate) fn effects(&self) -> &[EditorEffect] {
		&self.effects
	}
}

impl EditorBuffer {
	pub(crate) fn new(font_system: &mut FontSystem, text: impl Into<String>, config: SceneConfig) -> Self {
		let document = DocumentState::new(text);
		let layout = EditorLayout::new(font_system, document.text(), config);
		let mut editor = Self {
			document,
			session: EditorSession::new(),
			layout,
		};
		editor.reset_normal_selection();
		editor.refresh_view_state();
		editor
	}

	pub(crate) fn text(&self) -> &str {
		self.document.text()
	}

	pub(crate) fn mode(&self) -> EditorMode {
		self.session.mode()
	}

	pub(crate) fn buffer(&self) -> Arc<Buffer> {
		self.layout.buffer()
	}

	pub(crate) fn history_depths(&self) -> (usize, usize) {
		self.document.history_depths()
	}

	pub(crate) fn sync_buffer_config(&mut self, font_system: &mut FontSystem, config: SceneConfig) {
		let text = self.text().to_string();
		self.layout.sync_buffer_config(font_system, &text, config);
		self.refresh_view_state();
	}

	pub(crate) fn sync_buffer_width(&mut self, font_system: &mut FontSystem, width: f32) {
		self.layout.sync_buffer_width(font_system, width);
		self.refresh_view_state();
	}

	pub(crate) fn reset(&mut self, font_system: &mut FontSystem, text: impl Into<String>, config: SceneConfig) {
		self.document.reset(text);
		self.session = EditorSession::new();
		let text = self.text().to_string();
		self.layout.reset(font_system, &text, config);
		self.reset_normal_selection();
		self.refresh_view_state();
	}

	pub(crate) fn view_state(&self) -> EditorViewState {
		self.layout.view_state()
	}

	pub(crate) fn apply(&mut self, font_system: &mut FontSystem, command: EditorCommand) -> EditorUpdate {
		let previous_view = self.layout.view_state();
		let result = apply_command(self, font_system, command);
		self.refresh_view_state();
		EditorUpdate::from_apply_result(&previous_view, self.layout.view_state_ref(), result)
	}

	pub(crate) fn selection_details(&self) -> String {
		let (undo_depth, redo_depth) = self.document.history_depths();
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
					self.layout.view_state_ref().selection_rectangles.len(),
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
						self.layout.view_state_ref().selection_rectangles.len(),
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
		self.layout.buffer_text()
	}

	fn layout_snapshot(&self) -> BufferLayoutSnapshot {
		self.layout.snapshot(self.text())
	}

	fn reset_normal_selection(&mut self) {
		if let Some(selection) = self
			.layout_snapshot()
			.cluster(0)
			.map(|cluster| cluster.byte_range.clone())
		{
			let head = selection.start;
			self.session
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

		self.session.set_normal_selection(
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
		self.session.history_snapshot()
	}

	fn restore_snapshot(&mut self, snapshot: &EditorSnapshot) {
		self.session.restore_snapshot(snapshot, self.document.len());
	}

	fn apply_document_edit(&mut self, font_system: &mut FontSystem, edit: &TextEdit) -> TextEdit {
		self.apply_buffer_edit(font_system, edit);
		self.document.apply_edit(edit)
	}

	fn record_history(&mut self, forward: TextEdit, inverse: TextEdit, before: EditorSnapshot) {
		self.document.record_history(HistoryEntry {
			forward,
			inverse,
			before,
			after: self.history_snapshot(),
		});
	}

	fn active_viewport_target(&self, layout: &BufferLayoutSnapshot) -> Option<EditorSelectionRect> {
		self.active_selection(layout).map(|cluster| EditorSelectionRect {
			x: cluster.x,
			y: cluster.y,
			width: cluster.width.max(1.0),
			height: cluster.height.max(1.0),
		})
	}

	fn refresh_view_state(&mut self) {
		let layout = self.layout_snapshot();
		self.layout.set_view_state(EditorViewState {
			mode: self.mode(),
			#[cfg(test)]
			selection: self.selection_range(),
			#[cfg(test)]
			selection_head: self.selection().map(EditorSelection::head),
			selection_rectangles: self
				.selection()
				.map(|selection| layout.selection_rectangles(selection.range()))
				.unwrap_or_else(|| Arc::from([])),
			viewport_target: self.active_viewport_target(&layout),
		});
	}

	fn selection(&self) -> Option<&EditorSelection> {
		self.session.selection()
	}

	fn selection_range(&self) -> Option<Range<usize>> {
		self.selection().map(EditorSelection::range_cloned)
	}

	fn set_selection(&mut self, selection: Option<EditorSelection>) {
		self.session.set_selection(selection);
	}

	fn set_mode(&mut self, mode: EditorMode) {
		self.session.set_mode(mode);
	}

	fn caret(&self) -> usize {
		self.session.caret()
	}

	fn preferred_x(&self) -> Option<f32> {
		self.session.preferred_x()
	}

	fn set_preferred_x(&mut self, preferred_x: Option<f32>) {
		self.session.set_preferred_x(preferred_x);
	}

	fn pointer_anchor(&self) -> Option<usize> {
		self.session.pointer_anchor()
	}

	fn set_pointer_anchor(&mut self, pointer_anchor: Option<usize>) {
		self.session.set_pointer_anchor(pointer_anchor);
	}

	fn clear_pointer_anchor(&mut self) {
		self.set_pointer_anchor(None);
	}

	fn enter_insert_at(&mut self, caret: usize) {
		let layout = self.layout_snapshot();
		self.set_insert_head(&layout, caret);
	}

	fn buffer_hit(&self, point: Point) -> Option<cosmic_text::Cursor> {
		self.layout.hit(point)
	}

	fn apply_buffer_edit(&mut self, font_system: &mut FontSystem, edit: &TextEdit) {
		let text = self.text().to_string();
		self.layout.apply_edit(font_system, &text, edit);
	}

	fn insert_selection(&self, layout: &BufferLayoutSnapshot, head: usize) -> Option<EditorSelection> {
		// Insert mode keeps showing the cluster at the insertion head so entering
		// insert at a boundary does not visually jump to the previous cluster.
		layout
			.cluster_at_or_after(head)
			.or_else(|| layout.cluster_before(head))
			.and_then(|index| layout.cluster(index))
			.map(|cluster| EditorSelection::new(cluster.byte_range.clone(), head))
	}

	fn set_insert_head(&mut self, layout: &BufferLayoutSnapshot, head: usize) {
		self.session.enter_insert(self.insert_selection(layout, head));
	}
}
