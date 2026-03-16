use {
	super::{ApplyResult, EditorEngine, EditorMode, EditorSelection, TextEdit},
	crate::editor::{
		layout_state::edit_changes_line_structure,
		text::{clamp_char_boundary, next_char_boundary, previous_char_boundary},
	},
	cosmic_text::FontSystem,
};

impl EditorEngine {
	pub(super) fn undo(&mut self, font_system: &mut FontSystem) -> ApplyResult {
		let Some(entry) = self.core.document.undo() else {
			return ApplyResult::default();
		};

		self.apply_document_edit(
			font_system,
			&entry.inverse,
			edit_changes_line_structure(self.text(), &entry.inverse),
		);
		self.restore_snapshot(&entry.before);
		let next_layout = self.document_layout();

		ApplyResult {
			text_edit: Some(entry.inverse),
			layout: Some(next_layout),
			view_refreshed: false,
		}
	}

	pub(super) fn redo(&mut self, font_system: &mut FontSystem) -> ApplyResult {
		let Some(entry) = self.core.document.redo() else {
			return ApplyResult::default();
		};

		self.apply_document_edit(
			font_system,
			&entry.forward,
			edit_changes_line_structure(self.text(), &entry.forward),
		);
		self.restore_snapshot(&entry.after);
		let next_layout = self.document_layout();

		ApplyResult {
			text_edit: Some(entry.forward),
			layout: Some(next_layout),
			view_refreshed: false,
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
		let inverse = self.apply_document_edit(
			font_system,
			&text_edit,
			edit_changes_line_structure(self.text(), &text_edit),
		);
		self.set_mode(EditorMode::Normal);
		self.clear_pointer_anchor();
		let next_layout = self.document_layout();
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
			view_refreshed: false,
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
					range,
					inserted: String::new(),
				};
				let structural = edit_changes_line_structure(self.text(), &text_edit);
				let inverse = self.apply_document_edit(font_system, &text_edit, structural);
				self.set_preferred_x(None);
				self.clear_pointer_anchor();
				self.finish_insert_edit(before, text_edit, inverse, previous, structural)
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
				let structural = edit_changes_line_structure(self.text(), &text_edit);
				let inverse = self.apply_document_edit(font_system, &text_edit, structural);
				self.set_preferred_x(None);
				self.clear_pointer_anchor();
				self.finish_insert_edit(before, text_edit, inverse, self.caret(), structural)
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
		let next_head = caret + text_edit.inserted.len();
		let structural = edit_changes_line_structure(self.text(), &text_edit);
		let inverse = self.apply_document_edit(font_system, &text_edit, structural);
		self.set_preferred_x(None);
		self.clear_pointer_anchor();
		self.finish_insert_edit(before, text_edit, inverse, next_head, structural)
	}

	fn finish_insert_edit(
		&mut self, before: super::history::EditorSnapshot, text_edit: TextEdit, inverse: TextEdit, next_head: usize,
		structural: bool,
	) -> ApplyResult {
		let (layout, view_refreshed) = if structural {
			let next_layout = self.document_layout();
			self.set_insert_head(&next_layout, next_head);
			(Some(next_layout), false)
		} else {
			self.set_insert_head_fast(next_head);
			self.refresh_insert_view_state_fast();
			(None, true)
		};
		self.record_history(text_edit.clone(), inverse, before);
		ApplyResult {
			text_edit: Some(text_edit),
			layout,
			view_refreshed,
		}
	}
}
