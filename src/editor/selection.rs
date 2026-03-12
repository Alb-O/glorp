use iced::Point;

use super::{EditorBuffer, EditorMode};
use crate::editor::layout::BufferLayoutSnapshot;
use crate::editor::text::{is_word_char, next_char, previous_char};

impl EditorBuffer {
	pub(super) fn pointer_cluster_index(&self, layout: &BufferLayoutSnapshot, point: Point) -> Option<usize> {
		self.buffer
			.hit(point.x, point.y)
			.and_then(|cursor| layout.cluster_index_for_cursor(cursor))
			.or_else(|| {
				layout
					.nearest_cluster_at(point.y, point.x)
					.or_else(|| (!layout.clusters().is_empty()).then_some(0))
			})
	}

	pub(super) fn extend_pointer_selection(&mut self, layout: &BufferLayoutSnapshot, position: Point) {
		let Some(anchor_byte) = self.pointer_anchor else {
			return;
		};
		let Some(anchor_index) = layout
			.cluster_at_or_after(anchor_byte)
			.or_else(|| layout.cluster_before(anchor_byte.saturating_add(1)))
		else {
			return;
		};
		let Some(target_index) = self.pointer_cluster_index(layout, position) else {
			return;
		};

		self.select_range(layout, anchor_index, target_index);
	}

	fn select_range(&mut self, layout: &BufferLayoutSnapshot, anchor_index: usize, target_index: usize) {
		let Some(anchor) = layout.cluster(anchor_index) else {
			return;
		};
		let Some(target) = layout.cluster(target_index) else {
			return;
		};
		let start = anchor.byte_range.start.min(target.byte_range.start);
		let end = anchor.byte_range.end.max(target.byte_range.end);
		self.mode = EditorMode::Normal;
		self.selection = Some(start..end);
		self.caret = target.byte_range.start;
		self.preferred_x = Some(target.center_x());
	}

	pub(super) fn select_word_at(&mut self, layout: &BufferLayoutSnapshot, position: Point) {
		let Some(cluster_index) = self.pointer_cluster_index(layout, position) else {
			return;
		};
		let Some(cluster) = layout.cluster(cluster_index) else {
			return;
		};
		let range = self.word_range(cluster.byte_range.clone());
		self.mode = EditorMode::Normal;
		self.selection = Some(range.clone());
		self.caret = range.start;
		self.preferred_x = Some(cluster.center_x());
		self.pointer_anchor = None;
	}

	fn word_range(&self, fallback: std::ops::Range<usize>) -> std::ops::Range<usize> {
		let Some(slice) = self.text().get(fallback.clone()) else {
			return fallback;
		};

		if !slice.chars().any(is_word_char) {
			return fallback;
		}

		let mut start = fallback.start;
		while let Some((index, ch)) = previous_char(self.text(), start) {
			if !is_word_char(ch) {
				break;
			}
			start = index;
		}

		let mut end = fallback.end;
		while let Some((next, ch)) = next_char(self.text(), end) {
			if !is_word_char(ch) {
				break;
			}
			end = next;
		}

		start..end
	}
}
