mod build;
mod data;
mod font;
mod geometry;
mod inspect;
#[cfg(test)]
mod tests;
mod text;

use {
	self::inspect::SceneInspectCache,
	crate::types::{FontChoice, RenderMode, ShapingChoice, WrapChoice},
	iced::Font,
	std::sync::Arc,
};

#[cfg(test)]
pub(crate) use self::build::LayoutSceneTestSpec;
pub(crate) use self::{
	data::{ClusterInfo, GlyphInfo, InspectRunInfo, OutlinePath, PathCommand, PathPoint, RunInfo},
	font::{build_buffer, make_font_system, scene_config},
	text::debug_snippet,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SceneConfig {
	pub(crate) font_choice: FontChoice,
	pub(crate) shaping: ShapingChoice,
	pub(crate) wrapping: WrapChoice,
	pub(crate) render_mode: RenderMode,
	pub(crate) font_size: f32,
	pub(crate) line_height: f32,
	pub(crate) max_width: f32,
}

impl SceneConfig {
	pub(crate) fn font(self) -> Font {
		self.font_choice.to_iced_font()
	}
}

#[derive(Debug)]
pub(crate) struct LayoutSceneModel {
	config: SceneConfig,
	scene: LayoutScene,
}

#[derive(Debug, Clone)]
pub(crate) struct LayoutScene {
	pub(crate) text: Arc<str>,
	pub(crate) wrapping: WrapChoice,
	pub(crate) max_width: f32,
	pub(crate) measured_width: f32,
	pub(crate) measured_height: f32,
	pub(crate) glyph_count: usize,
	pub(crate) font_count: usize,
	pub(crate) runs: Arc<[RunInfo]>,
	pub(crate) clusters: Arc<[ClusterInfo]>,
	pub(crate) warnings: Arc<[String]>,
	pub(crate) draw_outlines: bool,
	inspect: Arc<SceneInspectCache>,
}
