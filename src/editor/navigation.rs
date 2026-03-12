use super::{EditorBuffer, EditorMode};
use crate::editor::layout::BufferLayoutSnapshot;
use crate::editor::text::{next_char_boundary, previous_char_boundary};

impl EditorBuffer {
	pub(super) fn move_left(&mut self, layout: &BufferLayoutSnapshot) {
		self.pointer_anchor = None;
		match self.mode {
			EditorMode::Normal => {
				let Some(current) = self.active_selection_index(layout) else {
					return;
				};

				if let Some(previous) = current.checked_sub(1) {
					self.select_cluster(layout, previous);
				}
			}
			EditorMode::Insert => {
				self.caret = previous_char_boundary(self.text(), self.caret).unwrap_or(0);
				self.preferred_x = None;
			}
		}
	}

	pub(super) fn move_right(&mut self, layout: &BufferLayoutSnapshot) {
		self.pointer_anchor = None;
		match self.mode {
			EditorMode::Normal => {
				let Some(current) = self.active_selection_index(layout) else {
					return;
				};

				if current + 1 < layout.clusters().len() {
					self.select_cluster(layout, current + 1);
				}
			}
			EditorMode::Insert => {
				self.caret = next_char_boundary(self.text(), self.caret).unwrap_or(self.text().len());
				self.preferred_x = None;
			}
		}
	}

	pub(super) fn move_vertical(&mut self, layout: &BufferLayoutSnapshot, direction: isize) {
		self.pointer_anchor = None;
		match self.mode {
			EditorMode::Normal => {
				let Some(current) = self.active_selection(layout) else {
					return;
				};
				let preferred_x = self.preferred_x.unwrap_or_else(|| current.center_x());
				let Some(target) = layout.nearest_cluster_on_adjacent_run(current.run_index, preferred_x, direction)
				else {
					return;
				};
				self.select_cluster(layout, target);
				self.preferred_x = Some(preferred_x);
			}
			EditorMode::Insert => {
				let caret = layout.caret_metrics(self.caret, self.config.line_height);
				let preferred_x = self.preferred_x.unwrap_or(caret.x);
				let Some(target) = layout.nearest_cluster_on_adjacent_run(caret.run_index, preferred_x, direction)
				else {
					return;
				};
				let cluster = &layout.clusters()[target];
				self.caret = if preferred_x > cluster.center_x() {
					cluster.byte_range.end
				} else {
					cluster.byte_range.start
				};
				self.preferred_x = Some(preferred_x);
			}
		}
	}

	pub(super) fn move_line_edge(&mut self, layout: &BufferLayoutSnapshot, to_start: bool) {
		self.pointer_anchor = None;
		match self.mode {
			EditorMode::Normal => {
				let Some(current) = self.active_selection(layout) else {
					return;
				};
				let target = if to_start {
					layout.first_cluster_in_run(current.run_index)
				} else {
					layout.last_cluster_in_run(current.run_index)
				};

				if let Some(target) = target {
					self.select_cluster(layout, target);
				}
			}
			EditorMode::Insert => {
				let caret = layout.caret_metrics(self.caret, self.config.line_height);
				let target = if to_start {
					layout
						.first_cluster_in_run(caret.run_index)
						.map(|index| layout.clusters()[index].byte_range.start)
						.unwrap_or(self.caret)
				} else {
					layout
						.last_cluster_in_run(caret.run_index)
						.map(|index| layout.clusters()[index].byte_range.end)
						.unwrap_or(self.caret)
				};

				self.caret = target;
				self.preferred_x = None;
			}
		}
	}

	pub(super) fn exit_insert(&mut self) {
		let layout = self.layout_snapshot();
		self.mode = EditorMode::Normal;
		self.preferred_x = None;
		self.pointer_anchor = None;

		self.selection = layout
			.cluster_before(self.caret)
			.or_else(|| layout.cluster_at_or_after(self.caret))
			.and_then(|index| layout.cluster(index))
			.map(|cluster| cluster.byte_range.clone());
	}
}
