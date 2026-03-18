use {
	super::{EditorMode, EditorSelection, TextEdit},
	std::collections::VecDeque,
};

const HISTORY_LIMIT: usize = 256;

#[derive(Debug, Clone)]
pub struct EditorSnapshot {
	pub mode: EditorMode,
	pub selection: Option<EditorSelection>,
	pub preferred_x: Option<f32>,
}

#[derive(Debug, Clone)]
pub struct HistoryEntry {
	pub forward: TextEdit,
	pub inverse: TextEdit,
	pub before: EditorSnapshot,
	pub after: EditorSnapshot,
}

#[derive(Debug, Clone, Default)]
pub struct EditorHistory {
	undo: VecDeque<HistoryEntry>,
	redo: VecDeque<HistoryEntry>,
}

impl EditorHistory {
	pub fn clear(&mut self) {
		self.undo.clear();
		self.redo.clear();
	}

	pub fn record(&mut self, entry: HistoryEntry) {
		if self.undo.len() == HISTORY_LIMIT {
			self.undo.pop_front();
		}

		self.undo.push_back(entry);
		self.redo.clear();
	}

	pub fn undo(&mut self) -> Option<HistoryEntry> {
		move_tail(&mut self.undo, &mut self.redo)
	}

	pub fn redo(&mut self) -> Option<HistoryEntry> {
		move_tail(&mut self.redo, &mut self.undo)
	}

	pub fn undo_len(&self) -> usize {
		self.undo.len()
	}

	pub fn redo_len(&self) -> usize {
		self.redo.len()
	}
}

fn move_tail(from: &mut VecDeque<HistoryEntry>, to: &mut VecDeque<HistoryEntry>) -> Option<HistoryEntry> {
	let entry = from.pop_back()?;
	to.push_back(entry.clone());
	Some(entry)
}
