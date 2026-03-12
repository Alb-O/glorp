use cosmic_text::{Edit as _, Editor as CosmicEditor, FontSystem};

use std::sync::Arc;

use super::{ApplyResult, EditorBuffer, EditorMode, TextEdit};
use crate::editor::text::{byte_to_cursor, clamp_char_boundary, next_char_boundary, previous_char_boundary};

impl EditorBuffer {
	pub(super) fn undo(&mut self, font_system: &mut FontSystem) -> ApplyResult {
		let Some(entry) = self.history.undo() else {
			return ApplyResult {
				changed: false,
				text_edit: None,
			};
		};

		self.apply_document_edit(font_system, &entry.inverse);
		self.restore_snapshot(&entry.before);

		ApplyResult {
			changed: true,
			text_edit: Some(entry.inverse),
		}
	}

	pub(super) fn redo(&mut self, font_system: &mut FontSystem) -> ApplyResult {
		let Some(entry) = self.history.redo() else {
			return ApplyResult {
				changed: false,
				text_edit: None,
			};
		};

		self.apply_document_edit(font_system, &entry.forward);
		self.restore_snapshot(&entry.after);

		ApplyResult {
			changed: true,
			text_edit: Some(entry.forward),
		}
	}

	pub(super) fn delete_selection(&mut self, font_system: &mut FontSystem) -> ApplyResult {
		let Some(selection) = self.selection.clone() else {
			return ApplyResult {
				changed: false,
				text_edit: None,
			};
		};

		let before = self.history_snapshot();
		let text_edit = TextEdit {
			range: selection.clone(),
			inserted: String::new(),
		};
		let inverse = self.apply_document_edit(font_system, &text_edit);
		self.mode = EditorMode::Normal;
		self.pointer_anchor = None;
		let next_layout = self.layout_snapshot();
		self.selection = next_layout
			.cluster_at_or_after(selection.start)
			.or_else(|| next_layout.cluster_before(selection.start))
			.and_then(|index| next_layout.cluster(index))
			.map(|cluster| cluster.byte_range.clone());
		self.caret = clamp_char_boundary(self.text(), selection.start);
		self.preferred_x = None;
		self.record_history(text_edit.clone(), inverse, before);

		ApplyResult {
			changed: true,
			text_edit: Some(text_edit),
		}
	}

	pub(super) fn backspace(&mut self, font_system: &mut FontSystem) -> ApplyResult {
		match self.mode {
			EditorMode::Normal => self.delete_selection(font_system),
			EditorMode::Insert => {
				let Some(previous) = previous_char_boundary(self.text(), self.caret) else {
					return ApplyResult {
						changed: false,
						text_edit: None,
					};
				};

				let before = self.history_snapshot();
				let range = previous..self.caret;
				let text_edit = TextEdit {
					range: range.clone(),
					inserted: String::new(),
				};
				let inverse = self.apply_document_edit(font_system, &text_edit);
				self.caret = previous;
				self.preferred_x = None;
				self.pointer_anchor = None;
				self.record_history(text_edit.clone(), inverse, before);

				ApplyResult {
					changed: true,
					text_edit: Some(text_edit),
				}
			}
		}
	}

	pub(super) fn delete_forward(&mut self, font_system: &mut FontSystem) -> ApplyResult {
		match self.mode {
			EditorMode::Normal => self.delete_selection(font_system),
			EditorMode::Insert => {
				let Some(next) = next_char_boundary(self.text(), self.caret) else {
					return ApplyResult {
						changed: false,
						text_edit: None,
					};
				};

				let before = self.history_snapshot();
				let text_edit = TextEdit {
					range: self.caret..next,
					inserted: String::new(),
				};
				let inverse = self.apply_document_edit(font_system, &text_edit);
				self.preferred_x = None;
				self.pointer_anchor = None;
				self.record_history(text_edit.clone(), inverse, before);

				ApplyResult {
					changed: true,
					text_edit: Some(text_edit),
				}
			}
		}
	}

	pub(super) fn insert_text(&mut self, font_system: &mut FontSystem, text: String) -> ApplyResult {
		if text.is_empty() {
			return ApplyResult {
				changed: false,
				text_edit: None,
			};
		}

		if !matches!(self.mode, EditorMode::Insert) {
			self.mode = EditorMode::Insert;
		}

		let before = self.history_snapshot();
		self.caret = clamp_char_boundary(self.text(), self.caret);
		let text_edit = TextEdit {
			range: self.caret..self.caret,
			inserted: text,
		};
		let inverse = self.apply_document_edit(font_system, &text_edit);
		self.caret += text_edit.inserted.len();
		self.preferred_x = None;
		self.pointer_anchor = None;
		self.record_history(text_edit.clone(), inverse, before);

		ApplyResult {
			changed: true,
			text_edit: Some(text_edit),
		}
	}

	pub(super) fn apply_buffer_edit(&mut self, font_system: &mut FontSystem, edit: &TextEdit) {
		let start = byte_to_cursor(self.text(), edit.range.start);
		let end = byte_to_cursor(self.text(), edit.range.end);
		let buffer = Arc::make_mut(&mut self.buffer);
		let mut editor = CosmicEditor::new(&mut *buffer);

		editor.set_cursor(start);
		if start != end {
			editor.delete_range(start, end);
			editor.set_cursor(start);
		}
		if !edit.inserted.is_empty() {
			let _ = editor.insert_at(start, &edit.inserted, None);
		}

		buffer.shape_until_scroll(font_system, false);
	}
}
