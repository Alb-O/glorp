mod build;
mod data;
mod dump;
mod font;
mod geometry;
mod inspect;
#[cfg(test)]
mod tests;
mod text;

use iced::Font;

use std::sync::Arc;

use crate::types::{FontChoice, RenderMode, ShapingChoice, WrapChoice};

use self::inspect::SceneInspectCache;

pub(crate) use self::data::{ClusterInfo, GlyphInfo, InspectRunInfo, OutlinePath, PathCommand, PathPoint, RunInfo};
pub(crate) use self::font::{build_buffer, make_font_system, scene_config};

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
	pub(crate) font_choice: FontChoice,
	pub(crate) shaping: ShapingChoice,
	pub(crate) wrapping: WrapChoice,
	pub(crate) render_mode: RenderMode,
	pub(crate) font_size: f32,
	pub(crate) line_height: f32,
	pub(crate) max_width: f32,
	pub(crate) measured_width: f32,
	pub(crate) measured_height: f32,
	pub(crate) glyph_count: usize,
	pub(crate) font_count: usize,
	pub(crate) runs: Arc<[RunInfo]>,
	pub(crate) clusters: Arc<[ClusterInfo]>,
	pub(crate) warnings: Arc<[String]>,
	pub(crate) draw_canvas_text: bool,
	pub(crate) draw_outlines: bool,
	inspect: Arc<SceneInspectCache>,
}
