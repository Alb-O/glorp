use std::{ops::Range, sync::Arc};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
	#[must_use]
	pub const fn center_x(&self) -> f32 {
		self.width.mul_add(0.5, self.x)
	}
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct LayoutCaretMetrics {
	pub run_index: usize,
	pub x: f32,
}
