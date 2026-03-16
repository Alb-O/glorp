use std::{ops::Range, sync::Arc};

#[derive(Debug, Clone)]
pub(crate) struct LayoutRun {
	pub(crate) line_index: usize,
	pub(crate) rtl: bool,
	pub(crate) baseline: f32,
	pub(crate) line_top: f32,
	pub(crate) line_height: f32,
	pub(crate) line_width: f32,
	pub(crate) cluster_range: Range<usize>,
	pub(crate) glyph_count: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct LayoutCluster {
	pub(crate) byte_range: Range<usize>,
	pub(crate) glyph_count: usize,
	pub(crate) run_index: usize,
	pub(crate) width: f32,
	pub(crate) x: f32,
	pub(crate) y: f32,
	pub(crate) height: f32,
	pub(crate) font_summary: Arc<str>,
}

impl LayoutCluster {
	pub(crate) fn center_x(&self) -> f32 {
		self.x + (self.width * 0.5)
	}
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct LayoutCaretMetrics {
	pub(crate) run_index: usize,
	pub(crate) x: f32,
}
