use {
	super::{DocumentLayout, LayoutCluster, debug_range},
	crate::{
		overlay::{LayoutRect, OverlayLayer, OverlayPrimitive, OverlayRectKind},
		types::CanvasTarget,
	},
	std::{ops::Range, sync::Arc},
};

impl DocumentLayout {
	pub fn target_details(&self, target: Option<CanvasTarget>) -> Option<Arc<str>> {
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

	pub fn inspect_overlay_primitives(
		&self, hovered_target: Option<CanvasTarget>, selected_target: Option<CanvasTarget>, layout_width: f32,
		show_hitboxes: bool,
	) -> Arc<[OverlayPrimitive]> {
		hovered_target
			.into_iter()
			.map(|target| (target, false))
			.chain(selected_target.into_iter().map(|target| (target, true)))
			.flat_map(|(target, selected)| {
				self.target_overlay_primitives(target, selected, layout_width, show_hitboxes)
			})
			.collect()
	}

	fn target_overlay_primitives(
		&self, target: CanvasTarget, selected: bool, layout_width: f32, show_hitboxes: bool,
	) -> impl Iterator<Item = OverlayPrimitive> {
		self.target_rect(target).into_iter().flat_map(move |rect| {
			let (rect, kind, hitbox_kind) = match target {
				CanvasTarget::Run(_) => (
					LayoutRect {
						width: layout_width.max(rect.width).max(1.0),
						..rect
					},
					if selected {
						OverlayRectKind::InspectRunSelected
					} else {
						OverlayRectKind::InspectRunHover
					},
					None,
				),
				CanvasTarget::Cluster(_) => (
					rect,
					if selected {
						OverlayRectKind::InspectGlyphSelected
					} else {
						OverlayRectKind::InspectGlyphHover
					},
					show_hitboxes.then_some(if selected {
						OverlayRectKind::InspectGlyphHitboxSelected
					} else {
						OverlayRectKind::InspectGlyphHitboxHover
					}),
				),
			};

			std::iter::once(OverlayPrimitive::scene_rect(rect, kind, OverlayLayer::OverText)).chain(
				hitbox_kind
					.into_iter()
					.map(move |kind| OverlayPrimitive::scene_rect(rect, kind, OverlayLayer::OverText)),
			)
		})
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
		debug_range(&self.text, range)
	}
}
