use std::ops::Range;

use super::EditorMode;
use super::history::EditorSnapshot;

#[derive(Debug, Clone)]
pub(super) struct EditorSession {
	mode: EditorMode,
	selection: Option<Range<usize>>,
	caret: usize,
	preferred_x: Option<f32>,
	pointer_anchor: Option<usize>,
}

impl EditorSession {
	pub(super) fn new() -> Self {
		Self {
			mode: EditorMode::Normal,
			selection: None,
			caret: 0,
			preferred_x: None,
			pointer_anchor: None,
		}
	}

	pub(super) fn mode(&self) -> EditorMode {
		self.mode
	}

	pub(super) fn selection(&self) -> Option<&Range<usize>> {
		self.selection.as_ref()
	}

	pub(super) fn selection_cloned(&self) -> Option<Range<usize>> {
		self.selection.clone()
	}

	pub(super) fn set_mode(&mut self, mode: EditorMode) {
		self.mode = mode;
	}

	pub(super) fn caret(&self) -> usize {
		self.caret
	}

	pub(super) fn preferred_x(&self) -> Option<f32> {
		self.preferred_x
	}

	pub(super) fn pointer_anchor(&self) -> Option<usize> {
		self.pointer_anchor
	}

	pub(super) fn enter_insert_at(&mut self, caret: usize) {
		self.mode = EditorMode::Insert;
		self.caret = caret;
		self.preferred_x = None;
		self.pointer_anchor = None;
	}

	pub(super) fn set_normal_selection(
		&mut self, selection: Range<usize>, caret: usize, preferred_x: Option<f32>, pointer_anchor: Option<usize>,
	) {
		self.mode = EditorMode::Normal;
		self.selection = Some(selection);
		self.caret = caret;
		self.preferred_x = preferred_x;
		self.pointer_anchor = pointer_anchor;
	}

	pub(super) fn set_selection(&mut self, selection: Option<Range<usize>>) {
		self.selection = selection;
	}

	pub(super) fn set_caret(&mut self, caret: usize) {
		self.caret = caret;
	}

	pub(super) fn set_preferred_x(&mut self, preferred_x: Option<f32>) {
		self.preferred_x = preferred_x;
	}

	pub(super) fn set_pointer_anchor(&mut self, pointer_anchor: Option<usize>) {
		self.pointer_anchor = pointer_anchor;
	}

	pub(super) fn history_snapshot(&self) -> EditorSnapshot {
		EditorSnapshot {
			mode: self.mode,
			selection: self.selection.clone(),
			caret: self.caret,
			preferred_x: self.preferred_x,
		}
	}

	pub(super) fn restore_snapshot(&mut self, snapshot: &EditorSnapshot, document_len: usize) {
		self.mode = snapshot.mode;
		self.selection = snapshot.selection.clone();
		self.caret = snapshot.caret.min(document_len);
		self.preferred_x = snapshot.preferred_x;
		self.pointer_anchor = None;
	}
}
