use cosmic_text::{Attrs, Buffer, Command, FontSystem, LayoutGlyph, Metrics, SwashCache, fontdb};
use iced::{Font, Point};

use std::fmt::Write as _;
use std::ops::Range;
use std::sync::{Arc, OnceLock};

use crate::types::{CanvasTarget, FontChoice, RenderMode, ShapingChoice, WrapChoice};

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

impl LayoutScene {
	#[allow(clippy::too_many_arguments)]
	pub(crate) fn build(
		font_system: &mut FontSystem, text: String, font_choice: FontChoice, shaping: ShapingChoice,
		wrapping: WrapChoice, font_size: f32, line_height: f32, max_width: f32, render_mode: RenderMode,
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
					inspect_glyphs.push(GlyphInfo::from_layout_glyph(
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
					rtl: run.rtl,
					baseline: run.line_y,
					line_top: run.line_top,
					line_height: run.line_height,
					line_width: run.line_w,
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
			font_choice: config.font_choice,
			shaping: config.shaping,
			wrapping: config.wrapping,
			render_mode: config.render_mode,
			font_size: config.font_size,
			line_height: config.line_height,
			max_width: config.max_width,
			measured_width,
			measured_height,
			glyph_count,
			font_count: inspect.font_names.len(),
			runs: runs.into(),
			clusters: clusters.into(),
			warnings: warnings.into(),
			draw_canvas_text: config.render_mode.draw_canvas_text(),
			draw_outlines,
			inspect,
		}
	}

	pub(crate) fn hit_test(&self, local: Point) -> Option<CanvasTarget> {
		if let Some(runs) = self.inspect.runs.get() {
			for (run_index, run) in runs.iter().enumerate() {
				for (glyph_index, glyph) in run.glyphs.iter().enumerate() {
					if contains_point(local, glyph.x, glyph.y, glyph.width.max(1.0), glyph.height.max(1.0)) {
						return Some(CanvasTarget::Glyph { run_index, glyph_index });
					}
				}
			}
		} else {
			for cluster in self.clusters.iter() {
				if contains_point(
					local,
					cluster.x,
					cluster.y,
					cluster.width.max(1.0),
					cluster.height.max(1.0),
				) {
					return Some(CanvasTarget::Glyph {
						run_index: cluster.run_index,
						glyph_index: cluster.glyph_start,
					});
				}
			}
		}

		for (run_index, run) in self.runs.iter().enumerate() {
			if contains_point(
				local,
				0.0,
				run.line_top,
				self.max_width.max(run.line_width).max(1.0),
				run.line_height.max(1.0),
			) {
				return Some(CanvasTarget::Run(run_index));
			}
		}

		None
	}

	pub(crate) fn inspect_runs(&self) -> &[InspectRunInfo] {
		if self.draw_outlines {
			return self
				.inspect
				.runs
				.get()
				.expect("outline runs should be built eagerly")
				.as_ref();
		}

		self.inspect
			.runs
			.get_or_init(|| build_inspect_runs(&self.inspect))
			.as_ref()
	}

	pub(crate) fn glyph(&self, run_index: usize, glyph_index: usize) -> Option<&GlyphInfo> {
		self.inspect_runs().get(run_index)?.glyphs.get(glyph_index)
	}

	pub(crate) fn clusters(&self) -> &[ClusterInfo] {
		&self.clusters
	}

	pub(crate) fn cluster(&self, index: usize) -> Option<&ClusterInfo> {
		self.clusters.get(index)
	}

	pub(crate) fn cluster_index_for_range(&self, range: &Range<usize>) -> Option<usize> {
		let index = self
			.clusters
			.binary_search_by_key(&range.start, |cluster| cluster.byte_range.start)
			.ok()?;
		(self.clusters[index].byte_range == *range).then_some(index)
	}

	pub(crate) fn cluster_index_for_target(&self, target: CanvasTarget) -> Option<usize> {
		match target {
			CanvasTarget::Run(run_index) => self.nearest_cluster_in_run(run_index, 0.0),
			CanvasTarget::Glyph { run_index, glyph_index } => self
				.clusters
				.iter()
				.enumerate()
				.find(|(_, cluster)| {
					cluster.run_index == run_index
						&& glyph_index >= cluster.glyph_start
						&& glyph_index < cluster.glyph_end
				})
				.map(|(index, _)| index),
		}
	}

	pub(crate) fn cluster_at_or_after(&self, byte: usize) -> Option<usize> {
		let index = self.clusters.partition_point(|cluster| cluster.byte_range.end <= byte);
		(index < self.clusters.len()).then_some(index)
	}

	pub(crate) fn cluster_before(&self, byte: usize) -> Option<usize> {
		self.clusters
			.partition_point(|cluster| cluster.byte_range.start < byte)
			.checked_sub(1)
	}

	pub(crate) fn dump_text(&self) -> String {
		let fonts_seen = collect_fonts_seen(&self.inspect.font_names);
		build_dump(
			&self.text,
			self.font_choice,
			self.shaping,
			self.wrapping,
			self.render_mode,
			self.font_size,
			self.line_height,
			self.max_width,
			self.measured_width,
			self.measured_height,
			self.glyph_count,
			&fonts_seen,
			self.inspect_runs(),
		)
	}

	pub(crate) fn nearest_cluster_in_run(&self, run_index: usize, preferred_x: f32) -> Option<usize> {
		let run = self.runs.get(run_index)?;
		if run.cluster_range.is_empty() {
			return None;
		}

		self.clusters[run.cluster_range.clone()]
			.iter()
			.enumerate()
			.min_by(|(_, a), (_, b)| {
				(a.center_x() - preferred_x)
					.abs()
					.total_cmp(&(b.center_x() - preferred_x).abs())
			})
			.map(|(offset, _)| run.cluster_range.start + offset)
	}

	pub(crate) fn target_details(&self, target: Option<CanvasTarget>) -> Option<String> {
		match target? {
			CanvasTarget::Run(run_index) => {
				let run = self.runs.get(run_index)?;
				Some(format!(
					"  kind: run\n  run index: {run_index}\n  source line: {}\n  rtl: {}\n  top: {:.1}\n  baseline: {:.1}\n  height: {:.1}\n  width: {:.1}\n  glyphs: {}",
					run.line_index,
					run.rtl,
					run.line_top,
					run.baseline,
					run.line_height,
					run.line_width,
					run.glyph_count,
				))
			}
			CanvasTarget::Glyph { run_index, glyph_index } => {
				let run = self.inspect_runs().get(run_index)?;
				let glyph = run.glyphs.get(glyph_index)?;
				Some(format!(
					"  kind: glyph\n  run index: {run_index}\n  glyph index: {glyph_index}\n  source line: {}\n  cluster: {}\n  bytes: {:?}\n  font: {}\n  glyph id: {}\n  x/y: {:.1}, {:.1}\n  w/h: {:.1}, {:.1}\n  size: {:.1}\n  x/y offset: {:.3}, {:.3}\n  outline: {}",
					run.line_index,
					self.debug_snippet(&glyph.cluster_range),
					glyph.cluster_range,
					glyph.font_name,
					glyph.glyph_id,
					glyph.x,
					glyph.y,
					glyph.width,
					glyph.height,
					glyph.font_size,
					glyph.x_offset,
					glyph.y_offset,
					glyph.outline.is_some(),
				))
			}
		}
	}

	pub(crate) fn cluster_preview(&self, cluster: &ClusterInfo) -> String {
		self.debug_snippet(&cluster.byte_range)
	}

	fn debug_snippet(&self, range: &Range<usize>) -> String {
		self.text
			.get(range.clone())
			.map(debug_snippet)
			.unwrap_or_else(|| "<invalid utf8 slice>".to_string())
	}
}

#[cfg(test)]
impl LayoutScene {
	pub(crate) fn new_for_test(
		text: impl Into<Arc<str>>, font_choice: FontChoice, shaping: ShapingChoice, wrapping: WrapChoice,
		render_mode: RenderMode, font_size: f32, line_height: f32, max_width: f32, measured_width: f32,
		measured_height: f32, glyph_count: usize, font_count: usize, runs: Vec<RunInfo>, clusters: Vec<ClusterInfo>,
	) -> Self {
		let text = text.into();

		Self {
			text: text.clone(),
			font_choice,
			shaping,
			wrapping,
			render_mode,
			font_size,
			line_height,
			max_width,
			measured_width,
			measured_height,
			glyph_count,
			font_count,
			runs: runs.into(),
			clusters: clusters.into(),
			warnings: Vec::new().into(),
			draw_canvas_text: render_mode.draw_canvas_text(),
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
	pub(crate) rtl: bool,
	pub(crate) baseline: f32,
	pub(crate) line_top: f32,
	pub(crate) line_height: f32,
	pub(crate) line_width: f32,
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
	pub(crate) outline: Option<OutlinePath>,
}

impl GlyphInfo {
	fn from_layout_glyph(
		glyph: &LayoutGlyph, line_byte_offset: usize, line_top: f32, line_height: f32, font_name: Arc<str>,
		outline: Option<OutlinePath>,
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
			outline,
		}
	}
}

#[derive(Debug)]
struct SceneInspectCache {
	buffer: Arc<Buffer>,
	line_byte_offsets: Arc<[usize]>,
	font_names: Arc<[(fontdb::ID, Arc<str>)]>,
	runs: OnceLock<Arc<[InspectRunInfo]>>,
}

#[derive(Debug, Clone)]
pub(crate) struct OutlinePath {
	pub(crate) commands: Vec<PathCommand>,
}

#[derive(Debug, Clone)]
pub(crate) enum PathCommand {
	MoveTo(PathPoint),
	LineTo(PathPoint),
	QuadTo(PathPoint, PathPoint),
	CurveTo(PathPoint, PathPoint, PathPoint),
	Close,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct PathPoint {
	pub(crate) x: f32,
	pub(crate) y: f32,
}

pub(crate) fn make_font_system() -> FontSystem {
	let mut font_system = FontSystem::new();
	let db = font_system.db_mut();
	db.set_monospace_family("JetBrains Mono");
	db.set_sans_serif_family("Noto Sans CJK SC");
	font_system
}

pub(crate) fn scene_config(
	font_choice: FontChoice, shaping: ShapingChoice, wrapping: WrapChoice, render_mode: RenderMode, font_size: f32,
	line_height: f32, max_width: f32,
) -> SceneConfig {
	SceneConfig {
		font_choice,
		shaping,
		wrapping,
		render_mode,
		font_size,
		line_height,
		max_width,
	}
}

pub(crate) fn build_buffer(font_system: &mut FontSystem, text: &str, config: SceneConfig) -> Buffer {
	let mut buffer = Buffer::new(font_system, Metrics::new(config.font_size, config.line_height));
	buffer.set_size(font_system, Some(config.max_width), None);
	buffer.set_wrap(font_system, config.wrapping.to_cosmic());
	buffer.set_text(
		font_system,
		text,
		&to_attributes(config.font()),
		config.shaping.to_cosmic(text),
		None,
	);
	buffer
}

fn contains_point(point: Point, x: f32, y: f32, width: f32, height: f32) -> bool {
	point.x >= x && point.x <= x + width && point.y >= y && point.y <= y + height
}

fn build_clusters(
	run_index: usize, line_byte_offset: usize, line_top: f32, line_height: f32, glyphs: &[LayoutGlyph],
) -> Vec<ClusterInfo> {
	let mut clusters = Vec::new();
	let mut current: Option<ClusterInfo> = None;

	for (glyph_index, glyph) in glyphs.iter().enumerate() {
		let byte_range = (line_byte_offset + glyph.start)..(line_byte_offset + glyph.end);
		let glyph_y = line_top + glyph.y;
		let glyph_height = glyph.line_height_opt.unwrap_or(line_height);

		match current.as_mut() {
			Some(cluster) if cluster.byte_range == byte_range => {
				cluster.width = (glyph.x + glyph.w - cluster.x).max(cluster.width);
				cluster.height = cluster.height.max(glyph_height);
				cluster.glyph_end = glyph_index + 1;
				cluster.y = cluster.y.min(glyph_y);
			}
			_ => {
				if let Some(cluster) = current.take() {
					clusters.push(cluster);
				}

				current = Some(ClusterInfo {
					run_index,
					glyph_start: glyph_index,
					glyph_end: glyph_index + 1,
					byte_range,
					x: glyph.x,
					y: glyph_y,
					width: glyph.w.max(1.0),
					height: glyph_height.max(1.0),
				});
			}
		}
	}

	if let Some(cluster) = current {
		clusters.push(cluster);
	}

	clusters
}

fn build_inspect_runs(inspect: &SceneInspectCache) -> Arc<[InspectRunInfo]> {
	inspect
		.buffer
		.layout_runs()
		.map(|run| {
			let line_byte_offset = inspect.line_byte_offsets[run.line_i];
			InspectRunInfo {
				line_index: run.line_i,
				rtl: run.rtl,
				baseline: run.line_y,
				line_top: run.line_top,
				line_height: run.line_height,
				line_width: run.line_w,
				glyphs: run
					.glyphs
					.iter()
					.map(|glyph| {
						GlyphInfo::from_layout_glyph(
							glyph,
							line_byte_offset,
							run.line_top,
							run.line_height,
							lookup_font_name(&inspect.font_names, glyph.font_id),
							None,
						)
					})
					.collect(),
			}
		})
		.collect()
}

fn collect_fonts_seen(font_names: &[(fontdb::ID, Arc<str>)]) -> Vec<String> {
	font_names.iter().map(|(_, name)| name.to_string()).collect()
}

fn font_name(font_system: &FontSystem, font_names: &mut Vec<(fontdb::ID, Arc<str>)>, font_id: fontdb::ID) -> Arc<str> {
	if let Some((_, name)) = font_names.iter().find(|(id, _)| *id == font_id) {
		return name.clone();
	}

	let name: Arc<str> = font_system
		.db()
		.face(font_id)
		.map(|face| face.post_script_name.as_str())
		.unwrap_or_else(|| "unknown-font")
		.into();
	font_names.push((font_id, name.clone()));
	name
}

fn lookup_font_name(font_names: &[(fontdb::ID, Arc<str>)], font_id: fontdb::ID) -> Arc<str> {
	font_names
		.iter()
		.find(|(id, _)| *id == font_id)
		.map(|(_, name)| name.clone())
		.unwrap_or_else(|| Arc::<str>::from("unknown-font"))
}

fn glyph_outline(
	swash_cache: &mut SwashCache, font_system: &mut FontSystem, glyph: &LayoutGlyph, baseline: f32,
) -> Option<OutlinePath> {
	let physical_glyph = glyph.physical((0.0, 0.0), 1.0);
	swash_cache
		.get_outline_commands(font_system, physical_glyph.cache_key)
		.map(|commands| OutlinePath {
			commands: commands
				.iter()
				.map(|command| match command {
					Command::MoveTo(point) => PathCommand::MoveTo(PathPoint {
						x: point.x + glyph.x + glyph.x_offset,
						y: -point.y + baseline + glyph.y_offset,
					}),
					Command::LineTo(point) => PathCommand::LineTo(PathPoint {
						x: point.x + glyph.x + glyph.x_offset,
						y: -point.y + baseline + glyph.y_offset,
					}),
					Command::QuadTo(control, to) => PathCommand::QuadTo(
						PathPoint {
							x: control.x + glyph.x + glyph.x_offset,
							y: -control.y + baseline + glyph.y_offset,
						},
						PathPoint {
							x: to.x + glyph.x + glyph.x_offset,
							y: -to.y + baseline + glyph.y_offset,
						},
					),
					Command::CurveTo(a, b, to) => PathCommand::CurveTo(
						PathPoint {
							x: a.x + glyph.x + glyph.x_offset,
							y: -a.y + baseline + glyph.y_offset,
						},
						PathPoint {
							x: b.x + glyph.x + glyph.x_offset,
							y: -b.y + baseline + glyph.y_offset,
						},
						PathPoint {
							x: to.x + glyph.x + glyph.x_offset,
							y: -to.y + baseline + glyph.y_offset,
						},
					),
					Command::Close => PathCommand::Close,
				})
				.collect(),
		})
}

fn to_attributes(font: Font) -> Attrs<'static> {
	Attrs::new()
		.family(to_family(font.family))
		.weight(to_weight(font.weight))
		.stretch(to_stretch(font.stretch))
		.style(to_style(font.style))
}

fn to_family(family: iced::font::Family) -> cosmic_text::Family<'static> {
	match family {
		iced::font::Family::Name(name) => cosmic_text::Family::Name(name),
		iced::font::Family::SansSerif => cosmic_text::Family::SansSerif,
		iced::font::Family::Serif => cosmic_text::Family::Serif,
		iced::font::Family::Cursive => cosmic_text::Family::Cursive,
		iced::font::Family::Fantasy => cosmic_text::Family::Fantasy,
		iced::font::Family::Monospace => cosmic_text::Family::Monospace,
	}
}

fn to_weight(weight: iced::font::Weight) -> cosmic_text::Weight {
	match weight {
		iced::font::Weight::Thin => cosmic_text::Weight::THIN,
		iced::font::Weight::ExtraLight => cosmic_text::Weight::EXTRA_LIGHT,
		iced::font::Weight::Light => cosmic_text::Weight::LIGHT,
		iced::font::Weight::Normal => cosmic_text::Weight::NORMAL,
		iced::font::Weight::Medium => cosmic_text::Weight::MEDIUM,
		iced::font::Weight::Semibold => cosmic_text::Weight::SEMIBOLD,
		iced::font::Weight::Bold => cosmic_text::Weight::BOLD,
		iced::font::Weight::ExtraBold => cosmic_text::Weight::EXTRA_BOLD,
		iced::font::Weight::Black => cosmic_text::Weight::BLACK,
	}
}

fn to_stretch(stretch: iced::font::Stretch) -> cosmic_text::Stretch {
	match stretch {
		iced::font::Stretch::UltraCondensed => cosmic_text::Stretch::UltraCondensed,
		iced::font::Stretch::ExtraCondensed => cosmic_text::Stretch::ExtraCondensed,
		iced::font::Stretch::Condensed => cosmic_text::Stretch::Condensed,
		iced::font::Stretch::SemiCondensed => cosmic_text::Stretch::SemiCondensed,
		iced::font::Stretch::Normal => cosmic_text::Stretch::Normal,
		iced::font::Stretch::SemiExpanded => cosmic_text::Stretch::SemiExpanded,
		iced::font::Stretch::Expanded => cosmic_text::Stretch::Expanded,
		iced::font::Stretch::ExtraExpanded => cosmic_text::Stretch::ExtraExpanded,
		iced::font::Stretch::UltraExpanded => cosmic_text::Stretch::UltraExpanded,
	}
}

fn to_style(style: iced::font::Style) -> cosmic_text::Style {
	match style {
		iced::font::Style::Normal => cosmic_text::Style::Normal,
		iced::font::Style::Italic => cosmic_text::Style::Italic,
		iced::font::Style::Oblique => cosmic_text::Style::Oblique,
	}
}

#[allow(clippy::too_many_arguments)]
fn build_dump(
	text_value: &str, font: FontChoice, shaping: ShapingChoice, wrapping: WrapChoice, render_mode: RenderMode,
	font_size: f32, line_height: f32, max_width: f32, measured_width: f32, measured_height: f32, glyph_count: usize,
	fonts_seen: &[String], runs: &[InspectRunInfo],
) -> String {
	let mut dump = String::new();

	let _ = writeln!(dump, "config");
	let _ = writeln!(dump, "  font: {font}");
	let _ = writeln!(dump, "  shaping: {shaping}");
	let _ = writeln!(dump, "  wrapping: {wrapping}");
	let _ = writeln!(dump, "  render mode: {render_mode}");
	let _ = writeln!(dump, "  text length: {} bytes", text_value.len());
	let _ = writeln!(dump, "  font size: {:.1}", font_size);
	let _ = writeln!(dump, "  line height: {:.1}", line_height);
	let _ = writeln!(dump, "  max width: {:.1}", max_width);
	let _ = writeln!(dump, "  measured width: {:.1}", measured_width);
	let _ = writeln!(dump, "  measured height: {:.1}", measured_height);
	let _ = writeln!(dump, "  runs: {}", runs.len());
	let _ = writeln!(dump, "  glyphs: {glyph_count}");
	let _ = writeln!(dump, "  fonts used: {}", fonts_seen.join(", "));
	let _ = writeln!(dump);

	let glyph_limit = 220usize;
	let mut emitted = 0usize;

	for (run_index, run) in runs.iter().enumerate() {
		let _ = writeln!(
			dump,
			"run {run_index}: line={} rtl={} top={:.1} baseline={:.1} height={:.1} width={:.1} glyphs={}",
			run.line_index,
			run.rtl,
			run.line_top,
			run.baseline,
			run.line_height,
			run.line_width,
			run.glyphs.len(),
		);

		for glyph in &run.glyphs {
			if emitted >= glyph_limit {
				let remaining = glyph_count.saturating_sub(emitted);
				let _ = writeln!(dump, "  ... truncated {remaining} more glyphs");
				return dump;
			}

			emitted += 1;
			let _ = writeln!(
				dump,
				"  glyph {}: cluster={} bytes={:?} font={} glyph_id={} x={:.1} y={:.1} w={:.1} h={:.1} size={:.1} x_off={:.3} y_off={:.3} outline={}",
				emitted - 1,
				text_value
					.get(glyph.cluster_range.clone())
					.map(debug_snippet)
					.unwrap_or_else(|| "<invalid utf8 slice>".to_string()),
				glyph.cluster_range,
				glyph.font_name,
				glyph.glyph_id,
				glyph.x,
				glyph.y,
				glyph.width,
				glyph.height,
				glyph.font_size,
				glyph.x_offset,
				glyph.y_offset,
				glyph.outline.is_some(),
			);
		}

		let _ = writeln!(dump);
	}

	dump
}

fn debug_snippet(text: &str) -> String {
	let escaped: String = text.chars().flat_map(char::escape_default).collect();

	if escaped.is_empty() {
		"<empty>".to_string()
	} else {
		format!("\"{escaped}\"")
	}
}

fn line_byte_offsets(text: &str) -> Vec<usize> {
	let mut offsets = vec![0];

	for (index, ch) in text.char_indices() {
		if ch == '\n' {
			offsets.push(index + ch.len_utf8());
		}
	}

	offsets
}

#[cfg(test)]
mod tests {
	use super::{LayoutScene, make_font_system, scene_config};
	use crate::types::{FontChoice, RenderMode, ShapingChoice, WrapChoice};

	#[test]
	fn scene_build_is_stable_for_unicode_replace() {
		let expected = "abX\n漢字\n最後".to_string();
		let config = scene_config(
			FontChoice::SansSerif,
			ShapingChoice::Advanced,
			WrapChoice::Word,
			RenderMode::CanvasOnly,
			22.0,
			30.0,
			320.0,
		);

		let mut rebuilt_font_system = make_font_system();
		let rebuilt = LayoutScene::build(
			&mut rebuilt_font_system,
			expected.clone(),
			config.font_choice,
			config.shaping,
			config.wrapping,
			config.font_size,
			config.line_height,
			config.max_width,
			config.render_mode,
		);

		assert_eq!(rebuilt.text.as_ref(), expected);
		assert!(rebuilt.glyph_count > 0);
		assert!(rebuilt.measured_width > 0.0);
		assert!(rebuilt.measured_height > 0.0);
	}
}
