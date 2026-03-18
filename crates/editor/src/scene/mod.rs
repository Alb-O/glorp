mod build;
mod data;
mod font;
mod geometry;
mod inspect;
#[cfg(test)]
mod tests;
mod text;

use {
	crate::types::{FontChoice, FontChoiceExt, ShapingChoice, WrapChoice},
	iced::Font,
	std::sync::Arc,
};

#[cfg(test)]
pub use self::build::DocumentLayoutTestSpec;
pub use self::{
	build::resolve_font_names_from_buffer,
	data::{LayoutCaretMetrics, LayoutCluster, LayoutRun},
	font::{build_buffer, make_font_system, scene_config},
	text::{debug_range, line_byte_offsets},
};

pub type FontNameMap = Arc<[(cosmic_text::fontdb::ID, Arc<str>)]>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SceneConfig {
	pub font_choice: FontChoice,
	pub shaping: ShapingChoice,
	pub wrapping: WrapChoice,
	pub font_size: f32,
	pub line_height: f32,
	pub max_width: f32,
}

impl SceneConfig {
	pub fn font(self) -> Font {
		self.font_choice.to_iced_font()
	}
}

#[derive(Debug, Clone)]
pub struct DocumentLayout {
	pub text: Arc<str>,
	pub wrapping: WrapChoice,
	pub max_width: f32,
	pub measured_width: f32,
	pub measured_height: f32,
	pub glyph_count: usize,
	pub cluster_count: usize,
	pub font_count: usize,
	pub runs: Arc<[LayoutRun]>,
	pub clusters: Arc<[LayoutCluster]>,
	pub line_byte_offsets: Arc<[usize]>,
	byte_order: Arc<[usize]>,
	pub warnings: Arc<[String]>,
}
