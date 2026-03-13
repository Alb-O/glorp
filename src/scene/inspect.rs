use cosmic_text::{Buffer, Command, FontSystem, LayoutGlyph, SwashCache, fontdb};

use std::ops::Range;
use std::sync::{Arc, OnceLock};

use crate::overlay::{LayoutRect, OverlayPrimitive, OverlayRectKind};
use crate::types::CanvasTarget;

use super::text::debug_snippet;
use super::{GlyphInfo, InspectRunInfo, LayoutScene, OutlinePath, PathCommand, PathPoint};

#[derive(Debug)]
pub(super) struct SceneInspectCache {
	pub(super) buffer: Arc<Buffer>,
	pub(super) line_byte_offsets: Arc<[usize]>,
	pub(super) font_names: Arc<[(fontdb::ID, Arc<str>)]>,
	pub(super) runs: OnceLock<Arc<[InspectRunInfo]>>,
}

impl LayoutScene {
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

	fn debug_snippet(&self, range: &Range<usize>) -> String {
		self.text
			.get(range.clone())
			.map(debug_snippet)
			.unwrap_or_else(|| "<invalid utf8 slice>".to_string())
	}

	pub(crate) fn inspect_overlay_primitives(
		&self, hovered_target: Option<CanvasTarget>, selected_target: Option<CanvasTarget>, layout_width: f32,
		show_hitboxes: bool,
	) -> Arc<[OverlayPrimitive]> {
		let mut overlays = Vec::new();

		if let Some(target) = hovered_target {
			overlays.extend(self.target_overlay_primitives(target, false, layout_width, show_hitboxes));
		}

		if let Some(target) = selected_target {
			overlays.extend(self.target_overlay_primitives(target, true, layout_width, show_hitboxes));
		}

		overlays.into()
	}

	fn target_overlay_primitives(
		&self, target: CanvasTarget, selected: bool, layout_width: f32, show_hitboxes: bool,
	) -> Vec<OverlayPrimitive> {
		match target {
			CanvasTarget::Run(run_index) => {
				let Some(run) = self.runs.get(run_index) else {
					return Vec::new();
				};

				vec![OverlayPrimitive::scene_rect(
					LayoutRect {
						x: 0.0,
						y: run.line_top,
						width: layout_width.max(run.line_width).max(1.0),
						height: run.line_height.max(1.0),
					},
					if selected {
						OverlayRectKind::InspectRunSelected
					} else {
						OverlayRectKind::InspectRunHover
					},
				)]
			}
			CanvasTarget::Glyph { run_index, glyph_index } => {
				let Some(rect) = self.target_rect(target) else {
					return Vec::new();
				};
				let mut overlays = vec![OverlayPrimitive::scene_rect(
					rect,
					if selected {
						OverlayRectKind::InspectGlyphSelected
					} else {
						OverlayRectKind::InspectGlyphHover
					},
				)];

				if show_hitboxes {
					overlays.push(OverlayPrimitive::scene_rect(
						rect,
						if selected {
							OverlayRectKind::InspectGlyphHitboxSelected
						} else {
							OverlayRectKind::InspectGlyphHitboxHover
						},
					));
				}

				if self.glyph(run_index, glyph_index).is_some() {
					return overlays;
				}

				overlays
			}
		}
	}

	fn target_rect(&self, target: CanvasTarget) -> Option<LayoutRect> {
		match target {
			CanvasTarget::Run(run_index) => {
				let run = self.runs.get(run_index)?;
				Some(LayoutRect {
					x: 0.0,
					y: run.line_top,
					width: self.max_width.max(run.line_width).max(1.0),
					height: run.line_height.max(1.0),
				})
			}
			CanvasTarget::Glyph { run_index, glyph_index } => self
				.glyph(run_index, glyph_index)
				.map(|glyph| LayoutRect {
					x: glyph.x,
					y: glyph.y,
					width: glyph.width.max(1.0),
					height: glyph.height.max(1.0),
				})
				.or_else(|| {
					self.cluster_index_for_target(target)
						.and_then(|index| self.cluster(index))
						.map(|cluster| LayoutRect {
							x: cluster.x,
							y: cluster.y,
							width: cluster.width.max(1.0),
							height: cluster.height.max(1.0),
						})
				}),
		}
	}
}

pub(super) fn build_inspect_runs(inspect: &SceneInspectCache) -> Arc<[InspectRunInfo]> {
	inspect
		.buffer
		.layout_runs()
		.map(|run| {
			let line_byte_offset = inspect.line_byte_offsets[run.line_i];
			InspectRunInfo {
				line_index: run.line_i,
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

pub(super) fn font_name(
	font_system: &FontSystem, font_names: &mut Vec<(fontdb::ID, Arc<str>)>, font_id: fontdb::ID,
) -> Arc<str> {
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

pub(super) fn glyph_outline(
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

fn lookup_font_name(font_names: &[(fontdb::ID, Arc<str>)], font_id: fontdb::ID) -> Arc<str> {
	font_names
		.iter()
		.find(|(id, _)| *id == font_id)
		.map(|(_, name)| name.clone())
		.unwrap_or_else(|| Arc::<str>::from("unknown-font"))
}
