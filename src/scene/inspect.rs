use {
	super::{DocumentLayout, LayoutCluster, debug_snippet},
	crate::{
		overlay::{LayoutRect, OverlayLayer, OverlayPrimitive, OverlayRectKind},
		types::CanvasTarget,
	},
	std::{ops::Range, sync::Arc},
};

impl DocumentLayout {
	pub(crate) fn target_details(&self, target: Option<CanvasTarget>) -> Option<Arc<str>> {
		match target? {
			CanvasTarget::Run(run_index) => self.runs.get(run_index).map(|run| {
				Arc::<str>::from(format!(
					"  kind: run\n  run index: {run_index}\n  source line: {}\n  rtl: {}\n  top: {:.1}\n  baseline: {:.1}\n  height: {:.1}\n  width: {:.1}\n  glyphs: {}\n  clusters: {}",
					run.line_index,
					run.rtl,
					run.line_top,
					run.baseline,
					run.line_height,
					run.line_width,
					run.glyph_count,
					run.cluster_range.len(),
				))
			}),
			CanvasTarget::Cluster(index) => self.cluster(index).map(|cluster| cluster_details(self, index, cluster)),
		}
	}

	pub(crate) fn inspect_overlay_primitives(
		&self, hovered_target: Option<CanvasTarget>, selected_target: Option<CanvasTarget>, layout_width: f32,
		show_hitboxes: bool,
	) -> Arc<[OverlayPrimitive]> {
		let mut overlays = Vec::with_capacity(4);

		for (target, selected) in [(hovered_target, false), (selected_target, true)] {
			if let Some(target) = target {
				self.append_target_overlay_primitives(&mut overlays, target, selected, layout_width, show_hitboxes);
			}
		}

		overlays.into()
	}

	fn append_target_overlay_primitives(
		&self, overlays: &mut Vec<OverlayPrimitive>, target: CanvasTarget, selected: bool, layout_width: f32,
		show_hitboxes: bool,
	) {
		match target {
			CanvasTarget::Run(run_index) => {
				let Some(run) = self.runs.get(run_index) else {
					return;
				};

				overlays.push(OverlayPrimitive::scene_rect(
					LayoutRect {
						x: 0.0,
						y: run.line_top,
						width: layout_width.max(run.line_width).max(1.0),
						height: run.line_height.max(1.0),
					},
					run_overlay_kind(selected),
					OverlayLayer::OverText,
				));
			}
			CanvasTarget::Cluster(index) => {
				let Some(rect) = self.target_rect(CanvasTarget::Cluster(index)) else {
					return;
				};
				overlays.push(OverlayPrimitive::scene_rect(
					rect,
					cluster_overlay_kind(selected),
					OverlayLayer::OverText,
				));

				if show_hitboxes {
					overlays.push(OverlayPrimitive::scene_rect(
						rect,
						cluster_hitbox_overlay_kind(selected),
						OverlayLayer::OverText,
					));
				}
			}
		}
	}
}

fn run_overlay_kind(selected: bool) -> OverlayRectKind {
	match selected {
		true => OverlayRectKind::InspectRunSelected,
		false => OverlayRectKind::InspectRunHover,
	}
}

fn cluster_overlay_kind(selected: bool) -> OverlayRectKind {
	match selected {
		true => OverlayRectKind::InspectGlyphSelected,
		false => OverlayRectKind::InspectGlyphHover,
	}
}

fn cluster_hitbox_overlay_kind(selected: bool) -> OverlayRectKind {
	match selected {
		true => OverlayRectKind::InspectGlyphHitboxSelected,
		false => OverlayRectKind::InspectGlyphHitboxHover,
	}
}

fn cluster_details(layout: &DocumentLayout, index: usize, cluster: &LayoutCluster) -> Arc<str> {
	Arc::<str>::from(format!(
		"  kind: cluster\n  cluster index: {index}\n  run index: {}\n  bytes: {:?}\n  text: {}\n  glyphs: {}\n  font: {}\n  x/y: {:.1}, {:.1}\n  w/h: {:.1}, {:.1}",
		cluster.run_index,
		cluster.byte_range,
		layout.debug_snippet(&cluster.byte_range),
		cluster.glyph_count,
		cluster.font_summary,
		cluster.x,
		cluster.y,
		cluster.width,
		cluster.height,
	))
}

impl DocumentLayout {
	fn debug_snippet(&self, range: &Range<usize>) -> String {
		self.text
			.get(range.start..range.end)
			.map_or_else(|| "<invalid utf8 slice>".to_string(), debug_snippet)
	}
}
