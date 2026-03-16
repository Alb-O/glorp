#[cfg(test)]
use cosmic_text::Metrics;
use {
	super::{
		LayoutScene, LayoutSceneModel, RunInfo, SceneConfig, geometry::count_clusters, inspect::SceneInspectCache,
		text::line_byte_offsets,
	},
	cosmic_text::{Buffer, FontSystem},
	std::{
		collections::BTreeSet,
		sync::{Arc, OnceLock},
	},
};

#[cfg(test)]
use super::build_buffer;

#[cfg(test)]
pub(crate) struct LayoutSceneTestSpec {
	pub(crate) text: Arc<str>,
	pub(crate) wrapping: crate::types::WrapChoice,
	pub(crate) font_size: f32,
	pub(crate) line_height: f32,
	pub(crate) max_width: f32,
	pub(crate) measured_width: f32,
	pub(crate) measured_height: f32,
	pub(crate) glyph_count: usize,
	pub(crate) font_count: usize,
	pub(crate) runs: Vec<RunInfo>,
	pub(crate) clusters: Vec<super::ClusterInfo>,
}

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
		font_system: &mut FontSystem, text: &str, font_choice: crate::types::FontChoice,
		shaping: crate::types::ShapingChoice, wrapping: crate::types::WrapChoice, font_size: f32, line_height: f32,
		max_width: f32,
	) -> Self {
		let config = SceneConfig {
			font_choice,
			shaping,
			wrapping,
			font_size,
			line_height,
			max_width,
		};

		let buffer = Arc::new(build_buffer(font_system, text, config));
		let model = LayoutSceneModel::new(font_system, text, buffer, config);
		model.scene
	}

	fn from_buffer(font_system: &mut FontSystem, text: &str, buffer: Arc<Buffer>, config: SceneConfig) -> Self {
		let mut runs = Vec::new();
		let mut warnings = Vec::new();
		let mut font_ids = BTreeSet::new();
		let mut measured_width: f32 = 0.0;
		let mut measured_height: f32 = 0.0;
		let mut glyph_count = 0usize;
		let mut cluster_count = 0usize;
		let line_byte_offsets = Arc::<[usize]>::from(line_byte_offsets(text));

		for run in buffer.layout_runs() {
			measured_width = measured_width.max(run.line_w);
			measured_height = measured_height.max(run.line_top + run.line_height);
			glyph_count += run.glyphs.len();
			let cluster_start = cluster_count;
			cluster_count += count_clusters(run.glyphs);

			font_ids.extend(run.glyphs.iter().map(|glyph| glyph.font_id));

			runs.push(RunInfo {
				line_index: run.line_i,
				rtl: run.rtl,
				baseline: run.line_y,
				line_top: run.line_top,
				line_height: run.line_height,
				line_width: run.line_w,
				cluster_range: cluster_start..cluster_count,
				glyph_count: run.glyphs.len(),
			});
		}

		let font_names = font_ids
			.into_iter()
			.map(|font_id| {
				let name = font_system
					.db()
					.face(font_id)
					.map_or_else(|| "unknown-font", |face| face.post_script_name.as_str());
				(font_id, Arc::<str>::from(name))
			})
			.collect::<Arc<[_]>>();

		if runs.is_empty() {
			warnings.push("No layout runs were produced. Check the font choice and text content.".to_string());
		}

		let inspect = Arc::new(SceneInspectCache {
			buffer,
			line_byte_offsets,
			font_names,
			clusters: OnceLock::new(),
			runs: OnceLock::new(),
			run_details: OnceLock::new(),
			glyph_details: OnceLock::new(),
		});

		Self {
			text: Arc::<str>::from(text),
			wrapping: config.wrapping,
			max_width: config.max_width,
			measured_width,
			measured_height,
			glyph_count,
			cluster_count,
			font_count: inspect.font_names.len(),
			runs: runs.into(),
			warnings: warnings.into(),
			inspect,
		}
	}
}

#[cfg(test)]
impl LayoutScene {
	pub(crate) fn new_for_test(spec: LayoutSceneTestSpec) -> Self {
		let LayoutSceneTestSpec {
			text,
			wrapping,
			font_size,
			line_height,
			max_width,
			measured_width,
			measured_height,
			glyph_count,
			font_count,
			runs,
			clusters,
		} = spec;

		Self {
			text: text.clone(),
			wrapping,
			max_width,
			measured_width,
			measured_height,
			glyph_count,
			cluster_count: clusters.len(),
			font_count,
			runs: runs.into(),
			warnings: Vec::new().into(),
			inspect: Arc::new(SceneInspectCache {
				buffer: Arc::new(Buffer::new_empty(Metrics::new(
					font_size.max(1.0),
					line_height.max(1.0),
				))),
				line_byte_offsets: Arc::from(line_byte_offsets(text.as_ref())),
				font_names: Vec::new().into(),
				clusters: {
					let lock = OnceLock::new();
					let _ = lock.set(clusters.into());
					lock
				},
				runs: OnceLock::new(),
				run_details: OnceLock::new(),
				glyph_details: OnceLock::new(),
			}),
		}
	}
}
