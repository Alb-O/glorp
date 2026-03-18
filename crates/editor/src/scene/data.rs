use std::{ops::Range, sync::Arc};

#[derive(Debug, Clone)]
pub struct LayoutRun {
	pub line_index: usize,
	pub rtl: bool,
	pub baseline: f32,
	pub line_top: f32,
	pub line_height: f32,
	pub line_width: f32,
	pub cluster_range: Range<usize>,
	pub glyph_count: usize,
}

#[derive(Debug, Clone)]
pub struct LayoutCluster {
	pub byte_range: Range<usize>,
	pub glyph_count: usize,
	pub run_index: usize,
	pub width: f32,
	pub x: f32,
	pub y: f32,
	pub height: f32,
	pub font_summary: Arc<str>,
}

impl LayoutCluster {
	pub fn center_x(&self) -> f32 {
		self.x + (self.width * 0.5)
	}
}

#[derive(Debug, Clone, Copy)]
pub struct LayoutCaretMetrics {
	pub run_index: usize,
	pub x: f32,
}
