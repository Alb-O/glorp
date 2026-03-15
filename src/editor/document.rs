use super::{
	TextEdit,
	history::{EditorHistory, HistoryEntry},
};

#[derive(Debug, Clone)]
pub(super) struct Document {
	text: String,
}

impl Document {
	pub(super) fn new(text: impl Into<String>) -> Self {
		Self { text: text.into() }
	}

	pub(super) fn text(&self) -> &str {
		&self.text
	}

	pub(super) fn len(&self) -> usize {
		self.text.len()
	}

	pub(super) fn is_empty(&self) -> bool {
		self.text.is_empty()
	}

	pub(super) fn reset(&mut self, text: impl Into<String>) {
		self.text = text.into();
	}

	pub(super) fn apply_edit(&mut self, edit: &TextEdit) -> TextEdit {
		let removed = self
			.text
			.get(edit.range.clone())
			.expect("text edit range should stay on char boundaries")
			.to_string();

		self.text.replace_range(edit.range.clone(), &edit.inserted);

		TextEdit {
			range: edit.range.start..(edit.range.start + edit.inserted.len()),
			inserted: removed,
		}
	}
}

#[derive(Debug, Clone)]
pub(super) struct DocumentState {
	document: Document,
	history: EditorHistory,
}

impl DocumentState {
	pub(super) fn new(text: impl Into<String>) -> Self {
		Self {
			document: Document::new(text),
			history: EditorHistory::default(),
		}
	}

	pub(super) fn text(&self) -> &str {
		self.document.text()
	}

	pub(super) fn len(&self) -> usize {
		self.document.len()
	}

	pub(super) fn is_empty(&self) -> bool {
		self.document.is_empty()
	}

	pub(super) fn reset(&mut self, text: impl Into<String>) {
		self.document.reset(text);
		self.history.clear();
	}

	pub(super) fn apply_edit(&mut self, edit: &TextEdit) -> TextEdit {
		self.document.apply_edit(edit)
	}

	pub(super) fn record_history(&mut self, entry: HistoryEntry) {
		self.history.record(entry);
	}

	pub(super) fn undo(&mut self) -> Option<HistoryEntry> {
		self.history.undo()
	}

	pub(super) fn redo(&mut self) -> Option<HistoryEntry> {
		self.history.redo()
	}

	pub(super) fn history_depths(&self) -> (usize, usize) {
		(self.history.undo_len(), self.history.redo_len())
	}
}
