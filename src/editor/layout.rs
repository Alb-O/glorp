use {
	crate::editor::text::line_byte_offsets,
	cosmic_text::{Buffer, Cursor, LayoutGlyph},
	std::ops::Range,
};

#[derive(Debug, Clone)]
pub(super) struct BufferRunInfo {
	cluster_range: Range<usize>,
	pub(super) line_height: f32,
	pub(super) line_top: f32,
}

#[derive(Debug, Clone)]
pub(super) struct BufferClusterInfo {
	pub(super) byte_range: Range<usize>,
	pub(super) height: f32,
	pub(super) run_index: usize,
	pub(super) width: f32,
	pub(super) x: f32,
	pub(super) y: f32,
}

impl BufferClusterInfo {
	pub(super) fn center_x(&self) -> f32 {
		self.x + (self.width * 0.5)
	}
}

#[derive(Debug, Clone)]
pub(super) struct BufferCaretMetrics {
	pub(super) run_index: usize,
	pub(super) x: f32,
}

#[derive(Debug, Clone)]
pub(super) struct BufferLayoutSnapshot {
	clusters: Vec<BufferClusterInfo>,
	byte_order: Vec<usize>,
	line_byte_offsets: Vec<usize>,
	runs: Vec<BufferRunInfo>,
}

impl BufferLayoutSnapshot {
	pub(super) fn new(buffer: &Buffer, text: &str) -> Self {
		let line_byte_offsets = line_byte_offsets(text);
		let mut runs = Vec::new();
		let mut clusters = Vec::new();

		for run in buffer.layout_runs() {
			let line_byte_offset = line_byte_offsets[run.line_i];
			let cluster_start = clusters.len();
			clusters.extend(build_buffer_clusters(
				runs.len(),
				line_byte_offset,
				run.line_top,
				run.line_height,
				run.glyphs,
			));
			let cluster_end = clusters.len();

			runs.push(BufferRunInfo {
				cluster_range: cluster_start..cluster_end,
				line_height: run.line_height,
				line_top: run.line_top,
			});
		}

		let mut byte_order = (0..clusters.len()).collect::<Vec<_>>();
		byte_order.sort_by(|a, b| {
			clusters[*a]
				.byte_range
				.start
				.cmp(&clusters[*b].byte_range.start)
				.then_with(|| clusters[*a].byte_range.end.cmp(&clusters[*b].byte_range.end))
				.then_with(|| clusters[*a].run_index.cmp(&clusters[*b].run_index))
		});

		Self {
			clusters,
			byte_order,
			line_byte_offsets,
			runs,
		}
	}

	pub(super) fn clusters(&self) -> &[BufferClusterInfo] {
		&self.clusters
	}

	pub(super) fn cluster(&self, index: usize) -> Option<&BufferClusterInfo> {
		self.clusters.get(index)
	}

	pub(super) fn cluster_index_for_cursor(&self, cursor: Cursor) -> Option<usize> {
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

	pub(super) fn cluster_at_or_after(&self, byte: usize) -> Option<usize> {
		let index = self
			.byte_order
			.partition_point(|cluster_index| self.clusters[*cluster_index].byte_range.end <= byte);
		self.byte_order.get(index).copied()
	}

	pub(super) fn cluster_before(&self, byte: usize) -> Option<usize> {
		self.byte_order
			.partition_point(|cluster_index| self.clusters[*cluster_index].byte_range.start < byte)
			.checked_sub(1)
			.and_then(|index| self.byte_order.get(index))
			.copied()
	}

	pub(super) fn first_cluster_in_run(&self, run_index: usize) -> Option<usize> {
		self.runs
			.get(run_index)
			.and_then(|run| (!run.cluster_range.is_empty()).then_some(run.cluster_range.start))
	}

	pub(super) fn last_cluster_in_run(&self, run_index: usize) -> Option<usize> {
		self.runs
			.get(run_index)
			.and_then(|run| (!run.cluster_range.is_empty()).then_some(run.cluster_range.end - 1))
	}

	pub(super) fn nearest_cluster_on_adjacent_run(
		&self, run_index: usize, preferred_x: f32, direction: isize,
	) -> Option<usize> {
		if direction < 0 {
			for next_run in (0..run_index).rev() {
				if let Some(target) = self.nearest_cluster_in_run(next_run, preferred_x) {
					return Some(target);
				}
			}
		} else {
			for next_run in (run_index.saturating_add(1))..self.runs.len() {
				if let Some(target) = self.nearest_cluster_in_run(next_run, preferred_x) {
					return Some(target);
				}
			}
		}

		None
	}

	fn nearest_cluster_in_run(&self, run_index: usize, preferred_x: f32) -> Option<usize> {
		let run = self.runs.get(run_index)?;
		if run.cluster_range.is_empty() {
			return None;
		}

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

	pub(super) fn nearest_cluster_at(&self, y: f32, preferred_x: f32) -> Option<usize> {
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

	pub(super) fn has_run_at_y(&self, y: f32) -> bool {
		self.runs.iter().any(|run| {
			let run_bottom = run.line_top + run.line_height;
			y >= run.line_top && y <= run_bottom
		})
	}

	pub(super) fn ends_hard_line(&self, byte: usize) -> bool {
		byte.checked_add(1)
			.is_some_and(|next| self.line_byte_offsets[1..].binary_search(&next).is_ok())
	}

	pub(super) fn cluster_at_insert_head(&self, byte: usize) -> Option<usize> {
		if self.ends_hard_line(byte) {
			return self.cluster_before(byte.saturating_add(1));
		}

		self.cluster_at_or_after(byte).or_else(|| self.cluster_before(byte))
	}

	pub(super) fn caret_metrics(&self, byte: usize) -> BufferCaretMetrics {
		if self.clusters.is_empty() {
			return BufferCaretMetrics { run_index: 0, x: 0.0 };
		}

		if let Some(index) = self.cluster_at_or_after(byte) {
			let cluster = &self.clusters[index];
			if byte <= cluster.byte_range.start {
				return BufferCaretMetrics {
					run_index: cluster.run_index,
					x: cluster.x,
				};
			}
		}

		if let Some(index) = self.cluster_before(byte) {
			let cluster = &self.clusters[index];
			return BufferCaretMetrics {
				run_index: cluster.run_index,
				x: cluster.x + cluster.width,
			};
		}

		BufferCaretMetrics { run_index: 0, x: 0.0 }
	}
}

fn build_buffer_clusters(
	run_index: usize, line_byte_offset: usize, line_top: f32, line_height: f32, glyphs: &[LayoutGlyph],
) -> Vec<BufferClusterInfo> {
	let mut clusters = Vec::with_capacity(glyphs.len());
	let mut current: Option<BufferClusterInfo> = None;

	for glyph in glyphs {
		let byte_range = (line_byte_offset + glyph.start)..(line_byte_offset + glyph.end);
		let glyph_y = line_top + glyph.y;
		let glyph_height = glyph.line_height_opt.unwrap_or(line_height);

		match current.as_mut() {
			Some(cluster) if cluster.byte_range == byte_range => {
				cluster.width = (glyph.x + glyph.w - cluster.x).max(cluster.width);
				cluster.height = cluster.height.max(glyph_height);
				cluster.y = cluster.y.min(glyph_y);
			}
			_ => {
				if let Some(cluster) = current.take() {
					clusters.push(cluster);
				}

				current = Some(BufferClusterInfo {
					byte_range,
					height: glyph_height.max(1.0),
					run_index,
					width: glyph.w.max(1.0),
					x: glyph.x,
					y: glyph_y,
				});
			}
		}
	}

	if let Some(cluster) = current {
		clusters.push(cluster);
	}

	clusters
}

fn run_distance(run: &BufferRunInfo, y: f32) -> f32 {
	let run_bottom = run.line_top + run.line_height;
	if y < run.line_top {
		run.line_top - y
	} else if y > run_bottom {
		y - run_bottom
	} else {
		0.0
	}
}
