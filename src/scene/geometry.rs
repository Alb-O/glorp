use {
	super::{ClusterInfo, LayoutScene},
	crate::types::CanvasTarget,
	cosmic_text::{Buffer, LayoutGlyph},
	iced::Point,
	std::sync::Arc,
};

impl LayoutScene {
	pub(crate) fn hit_test(&self, local: Point) -> Option<CanvasTarget> {
		if let Some(runs) = self.inspect.runs.get() {
			for (run_index, run) in runs.iter().enumerate() {
				for (glyph_index, glyph) in run.glyphs.iter().enumerate() {
					if contains_point(local, glyph.x, glyph.y, glyph.width.max(1.0), glyph.height.max(1.0)) {
						return Some(CanvasTarget::Glyph { run_index, glyph_index });
					}
				}
			}
		} else {
			for cluster in self.clusters() {
				if contains_point(
					local,
					cluster.x,
					cluster.y,
					cluster.width.max(1.0),
					cluster.height.max(1.0),
				) {
					return Some(CanvasTarget::Glyph {
						run_index: cluster.run_index,
						glyph_index: cluster.glyph_start,
					});
				}
			}
		}

		for (run_index, run) in self.runs.iter().enumerate() {
			if contains_point(
				local,
				0.0,
				run.line_top,
				self.max_width.max(run.line_width).max(1.0),
				run.line_height.max(1.0),
			) {
				return Some(CanvasTarget::Run(run_index));
			}
		}

		None
	}

	pub(crate) fn clusters(&self) -> &[ClusterInfo] {
		self.inspect
			.clusters
			.get_or_init(|| build_scene_clusters(&self.inspect.buffer, &self.inspect.line_byte_offsets))
			.as_ref()
	}

	pub(crate) fn cluster(&self, index: usize) -> Option<&ClusterInfo> {
		self.clusters().get(index)
	}

	pub(crate) fn cluster_index_for_target(&self, target: CanvasTarget) -> Option<usize> {
		match target {
			CanvasTarget::Run(run_index) => self.nearest_cluster_in_run(run_index, 0.0),
			CanvasTarget::Glyph { run_index, glyph_index } => {
				let run = self.runs.get(run_index)?;
				self.clusters()[run.cluster_range.clone()]
					.iter()
					.enumerate()
					.find(|(_, cluster)| glyph_index >= cluster.glyph_start && glyph_index < cluster.glyph_end)
					.map(|(offset, _)| run.cluster_range.start + offset)
			}
		}
	}

	pub(crate) fn nearest_cluster_in_run(&self, run_index: usize, preferred_x: f32) -> Option<usize> {
		let run = self.runs.get(run_index)?;
		if run.cluster_range.is_empty() {
			return None;
		}

		self.clusters()[run.cluster_range.clone()]
			.iter()
			.enumerate()
			.min_by(|(_, a), (_, b)| {
				(a.center_x() - preferred_x)
					.abs()
					.total_cmp(&(b.center_x() - preferred_x).abs())
			})
			.map(|(offset, _)| run.cluster_range.start + offset)
	}
}

pub(super) fn count_clusters(glyphs: &[LayoutGlyph]) -> usize {
	let mut count = 0usize;
	let mut current = None;

	for glyph in glyphs {
		let next = (glyph.start, glyph.end);
		if current != Some(next) {
			count += 1;
			current = Some(next);
		}
	}

	count
}

fn build_scene_clusters(buffer: &Buffer, line_byte_offsets: &[usize]) -> Arc<[ClusterInfo]> {
	buffer
		.layout_runs()
		.enumerate()
		.flat_map(|(run_index, run)| {
			build_clusters(
				run_index,
				line_byte_offsets[run.line_i],
				run.line_top,
				run.line_height,
				run.glyphs,
			)
		})
		.collect()
}

pub(super) fn build_clusters(
	run_index: usize, line_byte_offset: usize, line_top: f32, line_height: f32, glyphs: &[LayoutGlyph],
) -> Vec<ClusterInfo> {
	let mut clusters = Vec::with_capacity(glyphs.len());
	let mut current: Option<ClusterInfo> = None;

	for (glyph_index, glyph) in glyphs.iter().enumerate() {
		let byte_range = (line_byte_offset + glyph.start)..(line_byte_offset + glyph.end);
		let glyph_y = line_top + glyph.y;
		let glyph_height = glyph.line_height_opt.unwrap_or(line_height);

		match current.as_mut() {
			Some(cluster) if cluster.byte_range == byte_range => {
				cluster.width = (glyph.x + glyph.w - cluster.x).max(cluster.width);
				cluster.height = cluster.height.max(glyph_height);
				cluster.glyph_end = glyph_index + 1;
				cluster.y = cluster.y.min(glyph_y);
			}
			_ => {
				if let Some(cluster) = current.take() {
					clusters.push(cluster);
				}

				current = Some(ClusterInfo {
					run_index,
					glyph_start: glyph_index,
					glyph_end: glyph_index + 1,
					byte_range,
					x: glyph.x,
					y: glyph_y,
					width: glyph.w.max(1.0),
					height: glyph_height.max(1.0),
				});
			}
		}
	}

	if let Some(cluster) = current {
		clusters.push(cluster);
	}

	clusters
}

fn contains_point(point: Point, x: f32, y: f32, width: f32, height: f32) -> bool {
	point.x >= x && point.x <= x + width && point.y >= y && point.y <= y + height
}
