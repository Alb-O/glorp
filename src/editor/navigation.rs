use {
	super::{ApplyResult, EditorEngine, EditorMode},
	crate::{
		editor::text::{next_char_boundary, previous_char_boundary},
		scene::DocumentLayout,
	},
};

impl EditorEngine {
	pub(super) fn move_left(&mut self, layout: &DocumentLayout) {
		self.clear_pointer_anchor();
		match self.mode() {
			EditorMode::Normal => self.move_normal_selection(layout, |current| current.checked_sub(1)),
			EditorMode::Insert => {
				self.set_insert_head(
					layout,
					previous_char_boundary(self.text(), self.caret()).unwrap_or_default(),
				);
				self.set_preferred_x(None);
			}
		}
	}

	pub(super) fn move_right(&mut self, layout: &DocumentLayout) {
		self.clear_pointer_anchor();
		match self.mode() {
			EditorMode::Normal => self.move_normal_selection(layout, |current| {
				(current + 1 < layout.clusters().len()).then_some(current + 1)
			}),
			EditorMode::Insert => {
				self.set_insert_head(
					layout,
					next_char_boundary(self.text(), self.caret()).unwrap_or(self.text().len()),
				);
				self.set_preferred_x(None);
			}
		}
	}

	pub(super) fn move_vertical(&mut self, layout: &DocumentLayout, direction: isize) {
		self.clear_pointer_anchor();
		match self.mode() {
			EditorMode::Normal => {
				let Some(current) = self.active_selection(layout) else {
					return;
				};
				let preferred_x = self.preferred_x().unwrap_or(current.center_x());
				let Some(target) = layout.nearest_cluster_on_adjacent_run(current.run_index, preferred_x, direction)
				else {
					return;
				};
				self.select_cluster(layout, target);
				self.set_preferred_x(Some(preferred_x));
			}
			EditorMode::Insert => {
				let caret = layout.caret_metrics(self.caret());
				let preferred_x = self.preferred_x().unwrap_or(caret.x);
				let Some(target) = layout.nearest_cluster_on_adjacent_run(caret.run_index, preferred_x, direction)
				else {
					return;
				};
				let cluster = &layout.clusters()[target];
				self.set_insert_head(
					layout,
					if preferred_x > cluster.center_x() {
						cluster.byte_range.end
					} else {
						cluster.byte_range.start
					},
				);
				self.set_preferred_x(Some(preferred_x));
			}
		}
	}

	pub(super) fn move_line_edge(&mut self, layout: &DocumentLayout, to_start: bool) {
		self.clear_pointer_anchor();
		match self.mode() {
			EditorMode::Normal => {
				let Some(current) = self.active_selection(layout) else {
					return;
				};
				self.select_run_edge(layout, current.run_index, to_start);
			}
			EditorMode::Insert => {
				let caret = layout.caret_metrics(self.caret());
				let target = Self::run_edge_byte(layout, caret.run_index, to_start).unwrap_or(self.caret());

				self.set_insert_head(layout, target);
				self.set_preferred_x(None);
			}
		}
	}

	fn move_normal_selection(&mut self, layout: &DocumentLayout, target: impl FnOnce(usize) -> Option<usize>) {
		if let Some(target) = self.active_selection_index(layout).and_then(target) {
			self.select_cluster(layout, target);
		}
	}

	fn select_run_edge(&mut self, layout: &DocumentLayout, run_index: usize, to_start: bool) {
		if let Some(target) = Self::run_edge_index(layout, run_index, to_start) {
			self.select_cluster(layout, target);
		}
	}

	fn run_edge_index(layout: &DocumentLayout, run_index: usize, to_start: bool) -> Option<usize> {
		if to_start {
			layout.first_cluster_in_run(run_index)
		} else {
			layout.last_cluster_in_run(run_index)
		}
	}

	fn run_edge_byte(layout: &DocumentLayout, run_index: usize, to_start: bool) -> Option<usize> {
		Self::run_edge_index(layout, run_index, to_start).map(|index| {
			let range = &layout.clusters()[index].byte_range;
			if to_start { range.start } else { range.end }
		})
	}

	pub(super) fn exit_insert(&mut self) -> ApplyResult {
		if matches!(self.mode(), EditorMode::Normal) {
			self.set_preferred_x(None);
			self.clear_pointer_anchor();
			ApplyResult::default()
		} else {
			let layout = self.document_layout();
			self.set_mode(EditorMode::Normal);
			// Normal mode uses the same visible selection that insert mode showed, so
			// Esc does not shift the cursor left as a separate reconciliation step.
			let selection = Self::insert_selection(&layout, self.caret());
			self.set_selection(selection);
			self.set_preferred_x(None);
			self.clear_pointer_anchor();
			ApplyResult {
				text_edit: None,
				layout: Some(layout),
				view_refreshed: false,
			}
		}
	}
}
