use {
	super::{EditorEngine, EditorMode, EditorSelection},
	crate::{
		editor::text::{is_word_char, next_char, previous_char},
		scene::DocumentLayout,
	},
	iced::Point,
};

impl EditorEngine {
	pub fn pointer_cluster_index(&self, layout: &DocumentLayout, point: Point) -> Option<usize> {
		if !layout.has_run_at_y(point.y) {
			// `cosmic-text` can clamp far-away hits onto nearby text; ignore clicks that
			// are outside any laid-out line band so blank canvas space stays inert.
			return None;
		}

		self.buffer_hit(point)
			.and_then(|cursor| layout.cluster_index_for_cursor(cursor))
			.or_else(|| layout.nearest_cluster_at(point.y, point.x))
	}

	pub fn extend_pointer_selection(&mut self, layout: &DocumentLayout, position: Point) {
		let Some((anchor_index, target_index)) = self
			.pointer_anchor()
			.and_then(|anchor_byte| {
				layout
					.cluster_at_or_after(anchor_byte)
					.or_else(|| layout.cluster_before(anchor_byte.saturating_add(1)))
			})
			.zip(self.pointer_cluster_index(layout, position))
		else {
			return;
		};

		self.select_range(layout, anchor_index, target_index);
	}

	fn select_range(&mut self, layout: &DocumentLayout, anchor_index: usize, target_index: usize) {
		let Some((anchor, target)) = layout.cluster(anchor_index).zip(layout.cluster(target_index)) else {
			return;
		};
		let start = anchor.byte_range.start.min(target.byte_range.start);
		let end = anchor.byte_range.end.max(target.byte_range.end);
		self.set_mode(EditorMode::Normal);
		self.set_selection(Some(EditorSelection::new(start..end, target.byte_range.start)));
		self.set_preferred_x(Some(target.center_x()));
	}

	pub fn select_word_at(&mut self, layout: &DocumentLayout, position: Point) {
		let Some(cluster) = self
			.pointer_cluster_index(layout, position)
			.and_then(|cluster_index| layout.cluster(cluster_index))
		else {
			return;
		};
		let range = self.word_range(cluster.byte_range.clone());
		self.set_mode(EditorMode::Normal);
		let head = range.start;
		self.set_selection(Some(EditorSelection::new(range, head)));
		self.set_preferred_x(Some(cluster.center_x()));
		self.clear_pointer_anchor();
	}

	fn word_range(&self, fallback: std::ops::Range<usize>) -> std::ops::Range<usize> {
		let text = self.text();
		let Some(slice) = text.get(fallback.start..fallback.end) else {
			return fallback;
		};
		if !slice.chars().any(is_word_char) {
			return fallback;
		}

		let mut start = fallback.start;
		while let Some((index, ch)) = previous_char(text, start) {
			if !is_word_char(ch) {
				break;
			}
			start = index;
		}

		let mut end = fallback.end;
		while let Some((next, ch)) = next_char(text, end) {
			if !is_word_char(ch) {
				break;
			}
			end = next;
		}

		start..end
	}
}
