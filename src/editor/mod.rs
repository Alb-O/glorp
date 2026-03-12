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
	pub(crate) caret: usize,
	pub(crate) selection_rectangles: Arc<[EditorSelectionRect]>,
	pub(crate) caret_geometry: EditorCaretGeometry,
	pub(crate) viewport_target: Option<EditorSelectionRect>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct EditorSelectionRect {
	pub(crate) x: f32,
	pub(crate) y: f32,
	pub(crate) width: f32,
	pub(crate) height: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct EditorCaretGeometry {
	pub(crate) x: f32,
	pub(crate) y: f32,
	pub(crate) height: f32,
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
					selection,
					self.preview_range(selection),
					self.layout.view_state_ref().selection_rectangles.len(),
					self.caret(),
					self.pointer_anchor().unwrap_or(selection.start),
					undo_depth,
					redo_depth,
				)
			}
			EditorMode::Insert => format!(
				"  mode: {}\n  caret byte: {}\n  caret x/y: {:.1}, {:.1}\n  caret height: {:.1}\n  undo/redo: {}/{}",
				self.mode(),
				self.caret(),
				self.layout.view_state_ref().caret_geometry.x,
				self.layout.view_state_ref().caret_geometry.y,
				self.layout.view_state_ref().caret_geometry.height,
				undo_depth,
				redo_depth,
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
			let caret = selection.start;
			self.session.set_normal_selection(selection, caret, None, Some(caret));
		} else {
			self.set_selection(None);
			self.set_caret(0);
			self.clear_pointer_anchor();
		}
	}

	fn select_cluster(&mut self, layout: &BufferLayoutSnapshot, cluster_index: usize) {
		let Some(cluster) = layout.cluster(cluster_index) else {
			return;
		};

		self.session.set_normal_selection(
			cluster.byte_range.clone(),
			cluster.byte_range.start,
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
		match self.mode() {
			EditorMode::Normal => self.active_selection(layout).map(|cluster| EditorSelectionRect {
				x: cluster.x,
				y: cluster.y,
				width: cluster.width.max(1.0),
				height: cluster.height.max(1.0),
			}),
			EditorMode::Insert => {
				let caret = layout.caret_geometry(self.caret(), self.line_height());
				Some(EditorSelectionRect {
					x: caret.x,
					y: caret.y,
					width: 2.0,
					height: caret.height.max(1.0),
				})
			}
		}
	}

	fn refresh_view_state(&mut self) {
		let layout = self.layout_snapshot();
		self.layout.set_view_state(EditorViewState {
			mode: self.mode(),
			#[cfg(test)]
			selection: self.selection_range(),
			#[cfg(test)]
			caret: self.caret(),
			selection_rectangles: self
				.selection()
				.map(|selection| layout.selection_rectangles(selection))
				.unwrap_or_else(|| Arc::from([])),
			caret_geometry: layout.caret_geometry(self.caret(), self.line_height()),
			viewport_target: self.active_viewport_target(&layout),
		});
	}

	fn selection(&self) -> Option<&Range<usize>> {
		self.session.selection()
	}

	fn selection_range(&self) -> Option<Range<usize>> {
		self.session.selection_cloned()
	}

	fn set_selection(&mut self, selection: Option<Range<usize>>) {
		self.session.set_selection(selection);
	}

	fn set_mode(&mut self, mode: EditorMode) {
		self.session.set_mode(mode);
	}

	fn caret(&self) -> usize {
		self.session.caret()
	}

	fn set_caret(&mut self, caret: usize) {
		self.session.set_caret(caret);
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
		self.session.enter_insert_at(caret);
	}

	fn line_height(&self) -> f32 {
		self.layout.line_height()
	}

	fn buffer_hit(&self, point: Point) -> Option<cosmic_text::Cursor> {
		self.layout.hit(point)
	}

	fn apply_buffer_edit(&mut self, font_system: &mut FontSystem, edit: &TextEdit) {
		let text = self.text().to_string();
		self.layout.apply_edit(font_system, &text, edit);
	}
}
