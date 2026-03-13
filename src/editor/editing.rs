use cosmic_text::FontSystem;

use super::{ApplyResult, EditorEngine, EditorMode, EditorSelection, TextEdit};
use crate::editor::text::{clamp_char_boundary, next_char_boundary, previous_char_boundary};

impl EditorEngine {
	pub(super) fn undo(&mut self, font_system: &mut FontSystem) -> ApplyResult {
		let Some(entry) = self.state.document.undo() else {
			return ApplyResult::default();
		};

		self.apply_document_edit(font_system, &entry.inverse);
		self.restore_snapshot(&entry.before);
		let next_layout = self.layout_snapshot();

		ApplyResult {
			text_edit: Some(entry.inverse),
			layout: Some(next_layout),
		}
	}

	pub(super) fn redo(&mut self, font_system: &mut FontSystem) -> ApplyResult {
		let Some(entry) = self.state.document.redo() else {
			return ApplyResult::default();
		};

		self.apply_document_edit(font_system, &entry.forward);
		self.restore_snapshot(&entry.after);
		let next_layout = self.layout_snapshot();

		ApplyResult {
			text_edit: Some(entry.forward),
			layout: Some(next_layout),
		}
	}

	pub(super) fn delete_selection(&mut self, font_system: &mut FontSystem) -> ApplyResult {
		let Some(selection) = self.selection_range() else {
			return ApplyResult::default();
		};

		let before = self.history_snapshot();
		let text_edit = TextEdit {
			range: selection.clone(),
			inserted: String::new(),
		};
		let inverse = self.apply_document_edit(font_system, &text_edit);
		self.set_mode(EditorMode::Normal);
		self.clear_pointer_anchor();
		let next_layout = self.layout_snapshot();
		self.set_selection(
			next_layout
				.cluster_at_or_after(selection.start)
				.or_else(|| next_layout.cluster_before(selection.start))
				.and_then(|index| next_layout.cluster(index))
				.map(|cluster| EditorSelection::new(cluster.byte_range.clone(), cluster.byte_range.start)),
		);
		self.set_preferred_x(None);
		self.record_history(text_edit.clone(), inverse, before);

		ApplyResult {
			text_edit: Some(text_edit),
			layout: Some(next_layout),
		}
	}

	pub(super) fn backspace(&mut self, font_system: &mut FontSystem) -> ApplyResult {
		match self.mode() {
			EditorMode::Normal => self.delete_selection(font_system),
			EditorMode::Insert => {
				let Some(previous) = previous_char_boundary(self.text(), self.caret()) else {
					return ApplyResult::default();
				};

				let before = self.history_snapshot();
				let range = previous..self.caret();
				let text_edit = TextEdit {
					range: range.clone(),
					inserted: String::new(),
				};
				let inverse = self.apply_document_edit(font_system, &text_edit);
				let next_layout = self.layout_snapshot();
				self.set_insert_head(&next_layout, previous);
				self.set_preferred_x(None);
				self.clear_pointer_anchor();
				self.record_history(text_edit.clone(), inverse, before);

				ApplyResult {
					text_edit: Some(text_edit),
					layout: Some(next_layout),
				}
			}
		}
	}

	pub(super) fn delete_forward(&mut self, font_system: &mut FontSystem) -> ApplyResult {
		match self.mode() {
			EditorMode::Normal => self.delete_selection(font_system),
			EditorMode::Insert => {
				let Some(next) = next_char_boundary(self.text(), self.caret()) else {
					return ApplyResult::default();
				};

				let before = self.history_snapshot();
				let text_edit = TextEdit {
					range: self.caret()..next,
					inserted: String::new(),
				};
				let inverse = self.apply_document_edit(font_system, &text_edit);
				let next_layout = self.layout_snapshot();
				self.set_insert_head(&next_layout, self.caret());
				self.set_preferred_x(None);
				self.clear_pointer_anchor();
				self.record_history(text_edit.clone(), inverse, before);

				ApplyResult {
					text_edit: Some(text_edit),
					layout: Some(next_layout),
				}
			}
		}
	}

	pub(super) fn insert_text(&mut self, font_system: &mut FontSystem, text: String) -> ApplyResult {
		if text.is_empty() {
			return ApplyResult::default();
		}

		if !matches!(self.mode(), EditorMode::Insert) {
			self.enter_insert_at(self.caret());
		}

		let before = self.history_snapshot();
		let caret = clamp_char_boundary(self.text(), self.caret());
		let text_edit = TextEdit {
			range: caret..caret,
			inserted: text,
		};
		let inverse = self.apply_document_edit(font_system, &text_edit);
		let next_layout = self.layout_snapshot();
		self.set_insert_head(&next_layout, caret + text_edit.inserted.len());
		self.set_preferred_x(None);
		self.clear_pointer_anchor();
		self.record_history(text_edit.clone(), inverse, before);

		ApplyResult {
			text_edit: Some(text_edit),
			layout: Some(next_layout),
		}
	}
}
