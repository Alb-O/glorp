use {
	super::{EditorMode, EditorSelection, TextEdit},
	std::collections::VecDeque,
};

const HISTORY_LIMIT: usize = 256;

#[derive(Debug, Clone)]
pub(super) struct EditorSnapshot {
	pub(super) mode: EditorMode,
	pub(super) selection: Option<EditorSelection>,
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
	undo: VecDeque<HistoryEntry>,
	redo: VecDeque<HistoryEntry>,
}

impl EditorHistory {
	pub(super) fn clear(&mut self) {
		self.undo.clear();
		self.redo.clear();
	}

	pub(super) fn record(&mut self, entry: HistoryEntry) {
		if self.undo.len() == HISTORY_LIMIT {
			let _ = self.undo.pop_front();
		}

		self.undo.push_back(entry);
		self.redo.clear();
	}

	pub(super) fn undo(&mut self) -> Option<HistoryEntry> {
		let entry = self.undo.pop_back()?;
		self.redo.push_back(entry.clone());
		Some(entry)
	}

	pub(super) fn redo(&mut self) -> Option<HistoryEntry> {
		let entry = self.redo.pop_back()?;
		self.undo.push_back(entry.clone());
		Some(entry)
	}

	pub(super) fn undo_len(&self) -> usize {
		self.undo.len()
	}

	pub(super) fn redo_len(&self) -> usize {
		self.redo.len()
	}
}
