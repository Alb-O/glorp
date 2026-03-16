use {
	cosmic_text::LayoutGlyph,
	std::{ops::Range, sync::Arc},
};

#[derive(Debug, Clone)]
pub(crate) struct RunInfo {
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
pub(crate) struct InspectRunInfo {
	pub(crate) line_index: usize,
	pub(crate) glyphs: Vec<GlyphInfo>,
}

#[derive(Debug, Clone)]
pub(crate) struct ClusterInfo {
	pub(crate) run_index: usize,
	pub(crate) glyph_start: usize,
	pub(crate) glyph_end: usize,
	pub(crate) byte_range: Range<usize>,
	pub(crate) x: f32,
	pub(crate) y: f32,
	pub(crate) width: f32,
	pub(crate) height: f32,
}

impl ClusterInfo {
	pub(crate) fn center_x(&self) -> f32 {
		self.x + (self.width * 0.5)
	}
}

#[derive(Debug, Clone)]
pub(crate) struct GlyphInfo {
	pub(crate) cluster_range: Range<usize>,
	pub(crate) x: f32,
	pub(crate) y: f32,
	pub(crate) width: f32,
	pub(crate) height: f32,
	pub(crate) glyph_id: u16,
	pub(crate) font_name: Arc<str>,
	pub(crate) font_size: f32,
	pub(crate) x_offset: f32,
	pub(crate) y_offset: f32,
}

impl GlyphInfo {
	pub(super) fn from_layout_glyph(
		glyph: &LayoutGlyph, line_byte_offset: usize, line_top: f32, line_height: f32, font_name: Arc<str>,
	) -> Self {
		Self {
			cluster_range: (line_byte_offset + glyph.start)..(line_byte_offset + glyph.end),
			x: glyph.x,
			y: line_top + glyph.y,
			width: glyph.w,
			height: glyph.line_height_opt.unwrap_or(line_height),
			glyph_id: glyph.glyph_id,
			font_name,
			font_size: glyph.font_size,
			x_offset: glyph.x_offset,
			y_offset: glyph.y_offset,
		}
	}
}
