use {
	super::{DocumentLayout, LayoutCaretMetrics, LayoutCluster, LayoutRun},
	crate::{overlay::LayoutRect, types::CanvasTarget},
	cosmic_text::Cursor,
	iced::Point,
};

impl DocumentLayout {
	pub(crate) fn clusters(&self) -> &[LayoutCluster] {
		&self.clusters
	}

	pub(crate) fn cluster(&self, index: usize) -> Option<&LayoutCluster> {
		self.clusters.get(index)
	}

	pub(crate) fn cluster_at_or_after(&self, byte: usize) -> Option<usize> {
		let index = self
			.byte_order
			.partition_point(|cluster_index| self.clusters[*cluster_index].byte_range.end <= byte);
		self.byte_order.get(index).copied()
	}

	pub(crate) fn cluster_before(&self, byte: usize) -> Option<usize> {
		self.byte_order
			.partition_point(|cluster_index| self.clusters[*cluster_index].byte_range.start < byte)
			.checked_sub(1)
			.and_then(|index| self.byte_order.get(index))
			.copied()
	}

	pub(crate) fn cluster_index_for_cursor(&self, cursor: Cursor) -> Option<usize> {
		if self.clusters.is_empty() {
			return None;
		}

		let line_offset = self.line_byte_offsets.get(cursor.line).copied().unwrap_or_default();
		let byte = line_offset + cursor.index;

		if cursor.affinity.before() {
			self.clusters
				.iter()
				.enumerate()
				.find(|(_, cluster)| cluster.byte_range.end == byte)
				.map(|(index, _)| index)
				.or_else(|| self.cluster_before(byte.saturating_add(1)))
		} else {
			self.cluster_at_or_after(byte)
				.filter(|index| self.clusters[*index].byte_range.start <= byte)
				.or_else(|| self.cluster_before(byte))
		}
	}

	pub(crate) fn first_cluster_in_run(&self, run_index: usize) -> Option<usize> {
		self.runs
			.get(run_index)
			.and_then(|run| (!run.cluster_range.is_empty()).then_some(run.cluster_range.start))
	}

	pub(crate) fn last_cluster_in_run(&self, run_index: usize) -> Option<usize> {
		self.runs
			.get(run_index)
			.and_then(|run| (!run.cluster_range.is_empty()).then_some(run.cluster_range.end - 1))
	}

	pub(crate) fn nearest_cluster_in_run(&self, run_index: usize, preferred_x: f32) -> Option<usize> {
		let run = self.runs.get(run_index).filter(|run| !run.cluster_range.is_empty())?;
		self.clusters[run.cluster_range.clone()]
			.iter()
			.enumerate()
			.min_by(|(_, a), (_, b)| {
				(a.center_x() - preferred_x)
					.abs()
					.total_cmp(&(b.center_x() - preferred_x).abs())
			})
			.map(|(offset, _)| run.cluster_range.start + offset)
	}

	pub(crate) fn nearest_cluster_on_adjacent_run(
		&self, run_index: usize, preferred_x: f32, direction: isize,
	) -> Option<usize> {
		if direction < 0 {
			(0..run_index)
				.rev()
				.find_map(|next_run| self.nearest_cluster_in_run(next_run, preferred_x))
		} else {
			(run_index.saturating_add(1)..self.runs.len())
				.find_map(|next_run| self.nearest_cluster_in_run(next_run, preferred_x))
		}
	}

	pub(crate) fn nearest_cluster_at(&self, y: f32, preferred_x: f32) -> Option<usize> {
		let run_index = self
			.runs
			.iter()
			.enumerate()
			.min_by(|(_, a), (_, b)| {
				run_distance(a, y)
					.total_cmp(&run_distance(b, y))
					.then_with(|| a.line_top.total_cmp(&b.line_top))
			})
			.map(|(index, _)| index)?;
		self.nearest_cluster_in_run(run_index, preferred_x)
	}

	pub(crate) fn has_run_at_y(&self, y: f32) -> bool {
		self.runs
			.iter()
			.any(|run| y >= run.line_top && y <= run.line_top + run.line_height)
	}

	pub(crate) fn ends_hard_line(&self, byte: usize) -> bool {
		byte.checked_add(1)
			.is_some_and(|next| self.line_byte_offsets[1..].binary_search(&next).is_ok())
	}

	pub(crate) fn cluster_at_insert_head(&self, byte: usize) -> Option<usize> {
		// A caret parked on a hard newline should stay visually attached to the
		// preceding cluster instead of jumping onto the next rendered row.
		if self.ends_hard_line(byte) {
			return self.cluster_before(byte.saturating_add(1));
		}

		self.cluster_at_or_after(byte).or_else(|| self.cluster_before(byte))
	}

	pub(crate) fn caret_metrics(&self, byte: usize) -> LayoutCaretMetrics {
		self.cluster_at_or_after(byte)
			.and_then(|index| {
				let cluster = &self.clusters[index];
				(byte <= cluster.byte_range.start).then_some(LayoutCaretMetrics {
					run_index: cluster.run_index,
					x: cluster.x,
				})
			})
			.or_else(|| {
				self.cluster_before(byte).map(|index| {
					let cluster = &self.clusters[index];
					LayoutCaretMetrics {
						run_index: cluster.run_index,
						x: cluster.x + cluster.width,
					}
				})
			})
			.unwrap_or(LayoutCaretMetrics { run_index: 0, x: 0.0 })
	}

	pub(crate) fn hit_test(&self, local: Point) -> Option<CanvasTarget> {
		// Cluster hit boxes win over run bands so inspect mode stays precise when
		// both would match the same pointer position.
		self.clusters
			.iter()
			.enumerate()
			.find(|(_, cluster)| {
				contains_point(
					local,
					cluster.x,
					cluster.y,
					cluster.width.max(1.0),
					cluster.height.max(1.0),
				)
			})
			.map(|(index, _)| CanvasTarget::Cluster(index))
			.or_else(|| {
				self.runs.iter().enumerate().find_map(|(run_index, run)| {
					contains_point(
						local,
						0.0,
						run.line_top,
						self.max_width.max(run.line_width).max(1.0),
						run.line_height.max(1.0),
					)
					.then_some(CanvasTarget::Run(run_index))
				})
			})
	}

	pub(crate) fn target_rect(&self, target: CanvasTarget) -> Option<LayoutRect> {
		match target {
			CanvasTarget::Run(run_index) => self.runs.get(run_index).map(|run| LayoutRect {
				x: 0.0,
				y: run.line_top,
				width: self.max_width.max(run.line_width).max(1.0),
				height: run.line_height.max(1.0),
			}),
			CanvasTarget::Cluster(index) => self.cluster(index).map(cluster_rectangle),
		}
	}
}

pub(crate) fn cluster_rectangle(cluster: &LayoutCluster) -> LayoutRect {
	LayoutRect {
		x: cluster.x,
		y: cluster.y,
		width: cluster.width.max(1.0),
		height: cluster.height.max(1.0),
	}
}

fn run_distance(run: &LayoutRun, y: f32) -> f32 {
	let top = run.line_top;
	let bottom = run.line_top + run.line_height;

	if y < top {
		top - y
	} else if y > bottom {
		y - bottom
	} else {
		0.0
	}
}

fn contains_point(point: Point, x: f32, y: f32, width: f32, height: f32) -> bool {
	point.x >= x && point.x <= x + width && point.y >= y && point.y <= y + height
}
