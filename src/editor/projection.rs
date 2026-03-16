use {
	super::{
		EditorEngine, EditorMode, EditorSelection, EditorTextLayerState, EditorViewState, EditorViewportMetrics,
		TextEdit,
		geometry::{cluster_rectangle, insert_selection_range, normal_selection_geometry, selection_rectangles},
		layout_state::EditorLayout,
	},
	crate::{
		overlay::{EditorOverlayTone, LayoutRect, OverlayLayer, OverlayPrimitive, OverlayRectKind},
		scene::{DocumentLayout, SceneConfig},
		telemetry::duration_ms,
	},
	cosmic_text::{Buffer, FontSystem},
	iced::Point,
	std::{sync::Arc, time::Instant},
	tracing::{debug, trace},
};

#[derive(Debug, Clone)]
pub(super) struct EditorProjection {
	pub(super) layout: EditorLayout,
}

impl EditorProjection {
	pub(super) fn new(font_system: &mut FontSystem, text: &str, config: SceneConfig) -> Self {
		Self {
			layout: EditorLayout::new(font_system, text, config),
		}
	}
}

impl EditorEngine {
	pub(crate) fn buffer(&self) -> Arc<Buffer> {
		self.projection.layout.buffer()
	}

	pub(crate) fn sync_buffer_config(&mut self, font_system: &mut FontSystem, config: SceneConfig) -> bool {
		let Self { core, projection } = self;
		let changed = projection
			.layout
			.sync_buffer_config(font_system, core.document.text(), config);
		if changed {
			self.refresh_view_state(None);
		}
		changed
	}

	pub(crate) fn sync_buffer_width(&mut self, font_system: &mut FontSystem, width: f32) -> bool {
		let changed = self.projection.layout.sync_buffer_width(font_system, width);
		if changed {
			self.refresh_view_state_after_width_sync();
		}
		changed
	}

	pub(crate) fn view_state(&self) -> EditorViewState {
		self.projection.layout.view_state()
	}

	pub(crate) fn shared_document_layout(&self) -> Arc<DocumentLayout> {
		if let Some(layout) = self.projection.layout.cached_document_layout_arc() {
			return layout;
		}

		// Seed the retained snapshot on first demand so later presentation reads
		// observe one derived layout instead of rebuilding independently.
		let layout = Arc::new(self.document_layout());
		self.projection.layout.set_document_layout(layout.clone());
		layout
	}

	pub(crate) fn viewport_metrics(&self) -> EditorViewportMetrics {
		self.projection.layout.viewport_metrics()
	}

	pub(crate) fn text_layer_state(&self) -> EditorTextLayerState {
		self.projection.layout.text_layer_state()
	}

	#[cfg(test)]
	pub(crate) fn buffer_text(&self) -> String {
		self.projection.layout.buffer_text()
	}

	pub(super) fn document_layout(&self) -> DocumentLayout {
		let started = Instant::now();
		let document_layout = self.projection.layout.document_layout(self.text());
		let elapsed_ms = duration_ms(started.elapsed());
		if elapsed_ms >= 8.0 {
			debug!(
				duration_ms = elapsed_ms,
				clusters = document_layout.clusters.len(),
				text_bytes = self.text().len(),
				"document layout"
			);
		} else {
			trace!(
				duration_ms = elapsed_ms,
				clusters = document_layout.clusters.len(),
				text_bytes = self.text().len(),
				"document layout"
			);
		}
		document_layout
	}

	fn active_viewport_target(&self, layout: &DocumentLayout) -> Option<LayoutRect> {
		if matches!(self.mode(), EditorMode::Insert) {
			self.projection.layout.insert_cursor_block(self.text(), self.caret())
		} else {
			self.active_selection(layout).map(cluster_rectangle)
		}
	}

	pub(super) fn refresh_view_state(&mut self, layout: Option<DocumentLayout>) {
		let started = Instant::now();
		let layout = Arc::new(layout.unwrap_or_else(|| self.document_layout()));
		let layout_ref = layout.as_ref();
		let layout_elapsed = started.elapsed();
		let selection = self.selection().cloned();
		let selection_head = selection.as_ref().map(EditorSelection::head);
		let tone = EditorOverlayTone::from(self.mode());
		let insert_cursor = if matches!(self.mode(), EditorMode::Insert) {
			selection_head.and_then(|head| self.projection.layout.insert_cursor_rectangle(self.text(), head))
		} else {
			None
		};
		let viewport_target = self.active_viewport_target(layout_ref);
		let overlay_started = Instant::now();
		let overlays = self.build_overlays(layout_ref, selection.as_ref(), insert_cursor, viewport_target, tone);
		let overlay_elapsed = overlay_started.elapsed();
		self.projection.layout.set_view_state(EditorViewState {
			mode: self.mode(),
			selection: selection.as_ref().map(EditorSelection::range_cloned),
			selection_head,
			pointer_anchor: self.pointer_anchor(),
			overlays,
			viewport_target,
		});
		self.projection.layout.set_document_layout(layout);
		let total_ms = duration_ms(started.elapsed());
		if total_ms >= 8.0 {
			debug!(
				layout_ms = duration_ms(layout_elapsed),
				overlay_ms = duration_ms(overlay_elapsed),
				total_ms,
				text_bytes = self.text().len(),
				"refresh view state"
			);
		} else {
			trace!(
				layout_ms = duration_ms(layout_elapsed),
				overlay_ms = duration_ms(overlay_elapsed),
				total_ms,
				text_bytes = self.text().len(),
				"refresh view state"
			);
		}
	}

	fn refresh_view_state_after_width_sync(&mut self) {
		match self.mode() {
			EditorMode::Insert => self.refresh_insert_view_state_fast(),
			EditorMode::Normal => self.refresh_normal_view_state_fast(),
		}
	}

	fn build_overlays(
		&self, layout: &DocumentLayout, selection: Option<&EditorSelection>, insert_cursor: Option<LayoutRect>,
		viewport_target: Option<LayoutRect>, tone: EditorOverlayTone,
	) -> Arc<[OverlayPrimitive]> {
		let mut overlays = Vec::with_capacity(match self.mode() {
			EditorMode::Insert => 2,
			EditorMode::Normal => usize::from(viewport_target.is_some()),
		});

		if matches!(self.mode(), EditorMode::Insert) {
			if let Some(insert_block) = viewport_target {
				overlays.push(OverlayPrimitive::scene_rect(
					insert_block,
					OverlayRectKind::EditorInsertBlock(tone),
					OverlayLayer::UnderText,
				));
			}

			if let Some(caret) = insert_cursor {
				overlays.push(OverlayPrimitive::scene_rect(
					caret,
					OverlayRectKind::EditorCaret(tone),
					OverlayLayer::UnderText,
				));
			}
		} else {
			if let Some(selection) = selection {
				overlays.extend(
					selection_rectangles(layout, selection.range())
						.iter()
						.copied()
						.map(|rect| {
							OverlayPrimitive::scene_rect(
								rect,
								OverlayRectKind::EditorSelection(tone),
								OverlayLayer::UnderText,
							)
						}),
				);
			}

			if let Some(active) = viewport_target {
				overlays.push(OverlayPrimitive::scene_rect(
					active,
					OverlayRectKind::EditorActive(tone),
					OverlayLayer::UnderText,
				));
			}
		}

		overlays.into()
	}

	pub(super) fn buffer_hit(&self, point: Point) -> Option<cosmic_text::Cursor> {
		self.projection.layout.hit(point)
	}

	pub(super) fn apply_document_edit(
		&mut self, font_system: &mut FontSystem, edit: &TextEdit, structural: bool,
	) -> TextEdit {
		if structural {
			// Hard line breaks can reshuffle buffer segmentation, so the retained
			// buffer has to be rebuilt from whole-document text after the edit.
			let inverse = self.core.document.apply_edit(edit);
			self.rebuild_buffer(font_system, edit);
			return inverse;
		}

		self.apply_buffer_edit(font_system, edit);
		self.core.document.apply_edit(edit)
	}

	fn apply_buffer_edit(&mut self, font_system: &mut FontSystem, edit: &TextEdit) {
		let started = Instant::now();
		let Self { core, projection } = self;
		projection
			.layout
			.apply_incremental_edit(font_system, core.document.text(), edit);
		let total_ms = duration_ms(started.elapsed());
		if total_ms >= 8.0 {
			debug!(
				text_bytes = self.core.document.len(),
				total_ms,
				inserted_bytes = edit.inserted.len(),
				range_start = edit.range.start,
				range_end = edit.range.end,
				"apply buffer edit"
			);
		} else {
			trace!(
				text_bytes = self.core.document.len(),
				total_ms,
				inserted_bytes = edit.inserted.len(),
				range_start = edit.range.start,
				range_end = edit.range.end,
				"apply buffer edit"
			);
		}
	}

	fn rebuild_buffer(&mut self, font_system: &mut FontSystem, edit: &TextEdit) {
		let started = Instant::now();
		let Self { core, projection } = self;
		projection
			.layout
			.rebuild_buffer(font_system, core.document.text(), edit);
		let total_ms = duration_ms(started.elapsed());
		if total_ms >= 8.0 {
			debug!(
				text_bytes = self.core.document.len(),
				total_ms,
				inserted_bytes = edit.inserted.len(),
				range_start = edit.range.start,
				range_end = edit.range.end,
				"rebuild buffer"
			);
		} else {
			trace!(
				text_bytes = self.core.document.len(),
				total_ms,
				inserted_bytes = edit.inserted.len(),
				range_start = edit.range.start,
				range_end = edit.range.end,
				"rebuild buffer"
			);
		}
	}

	pub(super) fn set_insert_head_fast(&mut self, head: usize) {
		// Incremental insert edits can derive the visible caret cell straight from
		// the retained buffer and avoid a full scene-layout rebuild.
		let selection =
			insert_selection_range(&self.buffer(), self.text(), head).map(|range| EditorSelection::new(range, head));
		self.core.session.enter_insert(selection);
	}

	pub(super) fn refresh_insert_view_state_fast(&mut self) {
		let selection = self.selection().cloned();
		let selection_head = selection.as_ref().map(EditorSelection::head);
		let tone = EditorOverlayTone::from(self.mode());
		let insert_cursor =
			selection_head.and_then(|head| self.projection.layout.insert_cursor_rectangle(self.text(), head));
		let viewport_target =
			selection_head.and_then(|head| self.projection.layout.insert_cursor_block(self.text(), head));
		let mut overlays =
			Vec::with_capacity(usize::from(viewport_target.is_some()) + usize::from(insert_cursor.is_some()));

		if let Some(insert_block) = viewport_target {
			overlays.push(OverlayPrimitive::scene_rect(
				insert_block,
				OverlayRectKind::EditorInsertBlock(tone),
				OverlayLayer::UnderText,
			));
		}

		if let Some(caret) = insert_cursor {
			overlays.push(OverlayPrimitive::scene_rect(
				caret,
				OverlayRectKind::EditorCaret(tone),
				OverlayLayer::UnderText,
			));
		}

		self.projection.layout.set_view_state(EditorViewState {
			mode: self.mode(),
			selection: selection.as_ref().map(EditorSelection::range_cloned),
			selection_head,
			pointer_anchor: self.pointer_anchor(),
			overlays: overlays.into(),
			viewport_target,
		});
	}

	fn refresh_normal_view_state_fast(&mut self) {
		let selection = self.selection().cloned();
		let selection_head = selection.as_ref().map(EditorSelection::head);
		let tone = EditorOverlayTone::from(self.mode());
		let (selection_rectangles, viewport_target) = selection.as_ref().map_or_else(
			|| (Arc::from([]), None),
			|selection| {
				normal_selection_geometry(
					&self.buffer(),
					self.text(),
					selection.range(),
					selection_head.unwrap_or(selection.range().start),
				)
			},
		);
		let mut overlays = Vec::with_capacity(selection_rectangles.len() + usize::from(viewport_target.is_some()));

		overlays.extend(selection_rectangles.iter().copied().map(|rect| {
			OverlayPrimitive::scene_rect(rect, OverlayRectKind::EditorSelection(tone), OverlayLayer::UnderText)
		}));

		if let Some(active) = viewport_target {
			overlays.push(OverlayPrimitive::scene_rect(
				active,
				OverlayRectKind::EditorActive(tone),
				OverlayLayer::UnderText,
			));
		}

		self.projection.layout.set_view_state(EditorViewState {
			mode: self.mode(),
			selection: selection.as_ref().map(EditorSelection::range_cloned),
			selection_head,
			pointer_anchor: self.pointer_anchor(),
			overlays: overlays.into(),
			viewport_target,
		});
	}
}
