#[cfg(test)]
use cosmic_text::Metrics;
use {
	super::{
		InspectRunInfo, LayoutScene, LayoutSceneModel, RunInfo, SceneConfig,
		geometry::build_clusters,
		inspect::{SceneInspectCache, font_name, glyph_outline},
		text::line_byte_offsets,
	},
	cosmic_text::{Buffer, FontSystem, SwashCache},
	std::sync::{Arc, OnceLock},
};

#[cfg(test)]
use super::build_buffer;

impl LayoutSceneModel {
	pub(crate) fn new(font_system: &mut FontSystem, text: &str, buffer: Arc<Buffer>, config: SceneConfig) -> Self {
		let scene = LayoutScene::from_buffer(font_system, text, buffer, config);

		Self { config, scene }
	}

	pub(crate) fn scene(&self) -> &LayoutScene {
		&self.scene
	}

	pub(crate) fn rebuild(
		&mut self, font_system: &mut FontSystem, text: &str, buffer: Arc<Buffer>, config: SceneConfig,
	) {
		self.config = config;
		self.scene = LayoutScene::from_buffer(font_system, text, buffer, config);
	}
}

impl LayoutScene {
	#[cfg(test)]
	#[allow(clippy::too_many_arguments)]
	pub(crate) fn build(
		font_system: &mut FontSystem, text: String, font_choice: crate::types::FontChoice,
		shaping: crate::types::ShapingChoice, wrapping: crate::types::WrapChoice, font_size: f32, line_height: f32,
		max_width: f32, render_mode: crate::types::RenderMode,
	) -> Self {
		let config = SceneConfig {
			font_choice,
			shaping,
			wrapping,
			render_mode,
			font_size,
			line_height,
			max_width,
		};

		let buffer = Arc::new(build_buffer(font_system, &text, config));
		let model = LayoutSceneModel::new(font_system, &text, buffer, config);
		model.scene
	}

	fn from_buffer(font_system: &mut FontSystem, text: &str, buffer: Arc<Buffer>, config: SceneConfig) -> Self {
		let draw_outlines = config.render_mode.draw_outlines();
		let mut runs = Vec::new();
		let mut warnings = Vec::new();
		let mut font_names = Vec::new();
		let mut measured_width: f32 = 0.0;
		let mut measured_height: f32 = 0.0;
		let mut glyph_count = 0usize;
		let mut clusters = Vec::new();
		let line_byte_offsets = Arc::<[usize]>::from(line_byte_offsets(text));
		let mut eager_inspect_runs = draw_outlines.then(Vec::new);
		let mut swash_cache = draw_outlines.then(SwashCache::new);

		for run in buffer.layout_runs() {
			let line_byte_offset = line_byte_offsets[run.line_i];
			measured_width = measured_width.max(run.line_w);
			measured_height = measured_height.max(run.line_top + run.line_height);
			glyph_count += run.glyphs.len();
			let mut inspect_glyphs = draw_outlines.then(Vec::new);
			let cluster_start = clusters.len();
			clusters.extend(build_clusters(
				runs.len(),
				line_byte_offset,
				run.line_top,
				run.line_height,
				run.glyphs,
			));
			let cluster_end = clusters.len();

			if let Some(inspect_glyphs) = inspect_glyphs.as_mut() {
				for glyph in run.glyphs {
					let font_name = font_name(font_system, &mut font_names, glyph.font_id);
					let outline = swash_cache
						.as_mut()
						.and_then(|cache| glyph_outline(cache, font_system, glyph, run.line_y));
					inspect_glyphs.push(super::GlyphInfo::from_layout_glyph(
						glyph,
						line_byte_offset,
						run.line_top,
						run.line_height,
						font_name,
						outline,
					));
				}
			} else {
				for glyph in run.glyphs {
					let _ = font_name(font_system, &mut font_names, glyph.font_id);
				}
			}

			runs.push(RunInfo {
				line_index: run.line_i,
				rtl: run.rtl,
				baseline: run.line_y,
				line_top: run.line_top,
				line_height: run.line_height,
				line_width: run.line_w,
				cluster_range: cluster_start..cluster_end,
				glyph_count: run.glyphs.len(),
			});

			if let (Some(eager_runs), Some(glyphs)) = (eager_inspect_runs.as_mut(), inspect_glyphs) {
				eager_runs.push(InspectRunInfo {
					line_index: run.line_i,
					glyphs,
				});
			}
		}

		if runs.is_empty() {
			warnings.push("No layout runs were produced. Check the font choice and text content.".to_string());
		}

		let inspect = Arc::new(SceneInspectCache {
			buffer,
			line_byte_offsets,
			font_names: font_names.into(),
			runs: OnceLock::new(),
		});

		if let Some(eager_runs) = eager_inspect_runs {
			let _ = inspect.runs.set(eager_runs.into());
		}

		Self {
			text: Arc::<str>::from(text),
			wrapping: config.wrapping,
			max_width: config.max_width,
			measured_width,
			measured_height,
			glyph_count,
			font_count: inspect.font_names.len(),
			runs: runs.into(),
			clusters: clusters.into(),
			warnings: warnings.into(),
			draw_outlines,
			inspect,
		}
	}
}

#[cfg(test)]
impl LayoutScene {
	pub(crate) fn new_for_test(
		text: impl Into<Arc<str>>, _font_choice: crate::types::FontChoice, _shaping: crate::types::ShapingChoice,
		wrapping: crate::types::WrapChoice, render_mode: crate::types::RenderMode, font_size: f32, line_height: f32,
		max_width: f32, measured_width: f32, measured_height: f32, glyph_count: usize, font_count: usize,
		runs: Vec<RunInfo>, clusters: Vec<super::ClusterInfo>,
	) -> Self {
		let text = text.into();

		Self {
			text: text.clone(),
			wrapping,
			max_width,
			measured_width,
			measured_height,
			glyph_count,
			font_count,
			runs: runs.into(),
			clusters: clusters.into(),
			warnings: Vec::new().into(),
			draw_outlines: render_mode.draw_outlines(),
			inspect: Arc::new(SceneInspectCache {
				buffer: Arc::new(Buffer::new_empty(Metrics::new(
					font_size.max(1.0),
					line_height.max(1.0),
				))),
				line_byte_offsets: Arc::from(line_byte_offsets(text.as_ref())),
				font_names: Vec::new().into(),
				runs: OnceLock::new(),
			}),
		}
	}
}
