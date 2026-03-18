use {
	super::{ApplyResult, EditorEngine, EditorMode, EditorSelection, TextEdit},
	crate::{
		editor::{
			layout_state::edit_changes_line_structure,
			text::{clamp_char_boundary, next_char_boundary, previous_char_boundary},
		},
		scene::DocumentLayout,
	},
	cosmic_text::FontSystem,
};

impl EditorEngine {
	pub fn undo(&mut self, font_system: &mut FontSystem) -> ApplyResult {
		let entry = self.core.document.undo();
		self.apply_history_entry(font_system, entry, true)
	}

	pub fn redo(&mut self, font_system: &mut FontSystem) -> ApplyResult {
		let entry = self.core.document.redo();
		self.apply_history_entry(font_system, entry, false)
	}

	pub fn delete_selection(&mut self, font_system: &mut FontSystem) -> ApplyResult {
		let Some(selection) = self.selection_range() else {
			return ApplyResult::default();
		};

		let before = self.history_snapshot();
		let selection_start = selection.start;
		let text_edit = TextEdit {
			range: selection,
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
		self.set_selection(selection_near(&next_layout, selection_start));
		self.set_preferred_x(None);
		self.record_history(text_edit.clone(), inverse, before);

		ApplyResult {
			text_edit: Some(text_edit),
			layout: Some(next_layout),
			view_refreshed: false,
		}
	}

	pub fn backspace(&mut self, font_system: &mut FontSystem) -> ApplyResult {
		match self.mode() {
			EditorMode::Normal => self.delete_selection(font_system),
			EditorMode::Insert => {
				let Some(previous) = previous_char_boundary(self.text(), self.caret()) else {
					return ApplyResult::default();
				};
				self.delete_insert_range(font_system, previous..self.caret(), previous)
			}
		}
	}

	pub fn delete_forward(&mut self, font_system: &mut FontSystem) -> ApplyResult {
		match self.mode() {
			EditorMode::Normal => self.delete_selection(font_system),
			EditorMode::Insert => {
				let Some(next) = next_char_boundary(self.text(), self.caret()) else {
					return ApplyResult::default();
				};
				self.delete_insert_range(font_system, self.caret()..next, self.caret())
			}
		}
	}

	pub fn insert_text(&mut self, font_system: &mut FontSystem, text: String) -> ApplyResult {
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

	fn replay_history(
		&mut self, font_system: &mut FontSystem, text_edit: TextEdit, snapshot: &super::history::EditorSnapshot,
	) -> ApplyResult {
		self.apply_document_edit(
			font_system,
			&text_edit,
			edit_changes_line_structure(self.text(), &text_edit),
		);
		self.restore_snapshot(snapshot);
		ApplyResult {
			text_edit: Some(text_edit),
			layout: Some(self.document_layout()),
			view_refreshed: false,
		}
	}

	fn delete_insert_range(
		&mut self, font_system: &mut FontSystem, range: std::ops::Range<usize>, next_head: usize,
	) -> ApplyResult {
		let before = self.history_snapshot();
		let text_edit = TextEdit {
			range,
			inserted: String::new(),
		};
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

	fn apply_history_entry(
		&mut self, font_system: &mut FontSystem, entry: Option<super::history::HistoryEntry>, undo: bool,
	) -> ApplyResult {
		entry.map_or_else(ApplyResult::default, |entry| {
			let (text_edit, snapshot) = if undo {
				(entry.inverse, entry.before)
			} else {
				(entry.forward, entry.after)
			};
			self.replay_history(font_system, text_edit, &snapshot)
		})
	}
}

fn selection_near(layout: &DocumentLayout, byte: usize) -> Option<EditorSelection> {
	layout
		.cluster_at_or_after(byte)
		.or_else(|| layout.cluster_before(byte))
		.and_then(|index| layout.cluster(index))
		.map(|cluster| EditorSelection::new(cluster.byte_range.clone(), cluster.byte_range.start))
}
