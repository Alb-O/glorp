use cosmic_text::{Edit as _, Editor as CosmicEditor, FontSystem};

use std::sync::Arc;

use super::{ApplyResult, EditorBuffer, EditorMode, TextEdit};
use crate::editor::text::{byte_to_cursor, clamp_char_boundary, next_char_boundary, previous_char_boundary};

impl EditorBuffer {
	pub(super) fn delete_selection(&mut self, font_system: &mut FontSystem) -> ApplyResult {
		let Some(selection) = self.selection.clone() else {
			return ApplyResult {
				changed: false,
				text_edit: None,
			};
		};

		let text_edit = TextEdit {
			range: selection.clone(),
			inserted: String::new(),
		};
		self.apply_buffer_edit(font_system, &text_edit);
		self.text.replace_range(selection.clone(), "");
		self.mode = EditorMode::Normal;
		self.pointer_anchor = None;
		let next_layout = self.layout_snapshot();
		self.selection = next_layout
			.cluster_at_or_after(selection.start)
			.or_else(|| next_layout.cluster_before(selection.start))
			.and_then(|index| next_layout.cluster(index))
			.map(|cluster| cluster.byte_range.clone());
		self.caret = clamp_char_boundary(&self.text, selection.start);
		self.preferred_x = None;
		ApplyResult {
			changed: true,
			text_edit: Some(text_edit),
		}
	}

	pub(super) fn backspace(&mut self, font_system: &mut FontSystem) -> ApplyResult {
		match self.mode {
			EditorMode::Normal => self.delete_selection(font_system),
			EditorMode::Insert => {
				let Some(previous) = previous_char_boundary(&self.text, self.caret) else {
					return ApplyResult {
						changed: false,
						text_edit: None,
					};
				};

				let range = previous..self.caret;
				let text_edit = TextEdit {
					range: range.clone(),
					inserted: String::new(),
				};
				self.apply_buffer_edit(font_system, &text_edit);
				self.text.replace_range(previous..self.caret, "");
				self.caret = previous;
				self.preferred_x = None;
				self.pointer_anchor = None;
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
				let Some(next) = next_char_boundary(&self.text, self.caret) else {
					return ApplyResult {
						changed: false,
						text_edit: None,
					};
				};

				let range = self.caret..next;
				let text_edit = TextEdit {
					range: range.clone(),
					inserted: String::new(),
				};
				self.apply_buffer_edit(font_system, &text_edit);
				self.text.replace_range(self.caret..next, "");
				self.preferred_x = None;
				self.pointer_anchor = None;
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

		self.caret = clamp_char_boundary(&self.text, self.caret);
		let range = self.caret..self.caret;
		let text_edit = TextEdit {
			range: range.clone(),
			inserted: text,
		};
		self.apply_buffer_edit(font_system, &text_edit);
		self.text.insert_str(self.caret, &text_edit.inserted);
		self.caret += text_edit.inserted.len();
		self.preferred_x = None;
		self.pointer_anchor = None;
		ApplyResult {
			changed: true,
			text_edit: Some(text_edit),
		}
	}

	fn apply_buffer_edit(&mut self, font_system: &mut FontSystem, edit: &TextEdit) {
		let start = byte_to_cursor(&self.text, edit.range.start);
		let end = byte_to_cursor(&self.text, edit.range.end);
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
