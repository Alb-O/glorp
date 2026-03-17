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

		for (target, selected) in hovered_target
			.into_iter()
			.map(|target| (target, false))
			.chain(selected_target.into_iter().map(|target| (target, true)))
		{
			self.append_target_overlay_primitives(&mut overlays, target, selected, layout_width, show_hitboxes);
		}

		overlays.into()
	}

	fn append_target_overlay_primitives(
		&self, overlays: &mut Vec<OverlayPrimitive>, target: CanvasTarget, selected: bool, layout_width: f32,
		show_hitboxes: bool,
	) {
		let Some(rect) = self.target_rect(target) else {
			return;
		};
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

		overlays.push(OverlayPrimitive::scene_rect(rect, kind, OverlayLayer::OverText));

		if let Some(kind) = hitbox_kind {
			overlays.push(OverlayPrimitive::scene_rect(rect, kind, OverlayLayer::OverText));
		}
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
			.get(range.clone())
			.map_or_else(|| "<invalid utf8 slice>".to_string(), debug_snippet)
	}
}
