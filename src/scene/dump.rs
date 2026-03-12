use std::fmt::Write as _;

use super::inspect::collect_fonts_seen;
use super::text::debug_snippet;
use super::{InspectRunInfo, LayoutScene};
use crate::types::{FontChoice, RenderMode, ShapingChoice, WrapChoice};

impl LayoutScene {
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
