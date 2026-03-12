use super::TextEdit;

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
