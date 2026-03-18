mod build;
mod data;
mod font;
mod geometry;
mod inspect;
#[cfg(test)]
mod tests;
mod text;

use {
	crate::types::{FontChoice, ShapingChoice, WrapChoice},
	iced::Font,
	std::sync::Arc,
};

#[cfg(test)]
pub(crate) use self::build::DocumentLayoutTestSpec;
pub(crate) use self::{
	build::resolve_font_names_from_buffer,
	data::{LayoutCaretMetrics, LayoutCluster, LayoutRun},
	font::{build_buffer, make_font_system, scene_config},
	text::{debug_range, line_byte_offsets},
};

pub(crate) type FontNameMap = Arc<[(cosmic_text::fontdb::ID, Arc<str>)]>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SceneConfig {
	pub(crate) font_choice: FontChoice,
	pub(crate) shaping: ShapingChoice,
	pub(crate) wrapping: WrapChoice,
	pub(crate) font_size: f32,
	pub(crate) line_height: f32,
	pub(crate) max_width: f32,
}

impl SceneConfig {
	pub(crate) fn font(self) -> Font {
		self.font_choice.to_iced_font()
	}
}

#[derive(Debug, Clone)]
pub(crate) struct DocumentLayout {
	pub(crate) text: Arc<str>,
	pub(crate) wrapping: WrapChoice,
	pub(crate) max_width: f32,
	pub(crate) measured_width: f32,
	pub(crate) measured_height: f32,
	pub(crate) glyph_count: usize,
	pub(crate) cluster_count: usize,
	pub(crate) font_count: usize,
	pub(crate) runs: Arc<[LayoutRun]>,
	pub(crate) clusters: Arc<[LayoutCluster]>,
	pub(crate) line_byte_offsets: Arc<[usize]>,
	byte_order: Arc<[usize]>,
	pub(crate) warnings: Arc<[String]>,
}
