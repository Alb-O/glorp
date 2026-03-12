use std::ops::Range;

use super::{EditorMode, TextEdit};

const HISTORY_LIMIT: usize = 256;

#[derive(Debug, Clone)]
pub(super) struct EditorSnapshot {
	pub(super) mode: EditorMode,
	pub(super) selection: Option<Range<usize>>,
	pub(super) caret: usize,
	pub(super) preferred_x: Option<f32>,
}

#[derive(Debug, Clone)]
pub(super) struct HistoryEntry {
	pub(super) forward: TextEdit,
	pub(super) inverse: TextEdit,
	pub(super) before: EditorSnapshot,
	pub(super) after: EditorSnapshot,
}

#[derive(Debug, Clone, Default)]
pub(super) struct EditorHistory {
	undo: Vec<HistoryEntry>,
	redo: Vec<HistoryEntry>,
}

impl EditorHistory {
	pub(super) fn clear(&mut self) {
		self.undo.clear();
		self.redo.clear();
	}

	pub(super) fn record(&mut self, entry: HistoryEntry) {
		if self.undo.len() == HISTORY_LIMIT {
			let _ = self.undo.remove(0);
		}

		self.undo.push(entry);
		self.redo.clear();
	}

	pub(super) fn undo(&mut self) -> Option<HistoryEntry> {
		let entry = self.undo.pop()?;
		self.redo.push(entry.clone());
		Some(entry)
	}

	pub(super) fn redo(&mut self) -> Option<HistoryEntry> {
		let entry = self.redo.pop()?;
		self.undo.push(entry.clone());
		Some(entry)
	}

	pub(super) fn undo_len(&self) -> usize {
		self.undo.len()
	}

	pub(super) fn redo_len(&self) -> usize {
		self.redo.len()
	}
}
