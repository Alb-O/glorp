use super::{
	TextEdit,
	history::{EditorHistory, HistoryEntry},
};

#[derive(Debug, Clone)]
pub struct DocumentState {
	text: String,
	history: EditorHistory,
}

impl DocumentState {
	pub fn new(text: impl Into<String>) -> Self {
		Self {
			text: text.into(),
			history: EditorHistory::default(),
		}
	}

	pub fn text(&self) -> &str {
		&self.text
	}

	pub const fn len(&self) -> usize {
		self.text.len()
	}

	pub const fn is_empty(&self) -> bool {
		self.text.is_empty()
	}

	pub fn reset(&mut self, text: impl Into<String>) {
		self.text = text.into();
		self.history.clear();
	}

	pub fn apply_edit(&mut self, edit: &TextEdit) -> TextEdit {
		let range = edit.range.clone();
		let removed = self
			.text
			.get(range.clone())
			.expect("text edit range should stay on char boundaries")
			.to_string();

		self.text.replace_range(range, &edit.inserted);

		// History wants the inverse edit in post-apply coordinates so undo can
		// replay it directly against the updated document.
		TextEdit {
			range: edit.range.start..(edit.range.start + edit.inserted.len()),
			inserted: removed,
		}
	}

	pub fn record_history(&mut self, entry: HistoryEntry) {
		self.history.record(entry);
	}

	pub fn undo(&mut self) -> Option<HistoryEntry> {
		self.history.undo()
	}

	pub fn redo(&mut self) -> Option<HistoryEntry> {
		self.history.redo()
	}

	pub fn history_depths(&self) -> (usize, usize) {
		(self.history.undo_len(), self.history.redo_len())
	}
}
