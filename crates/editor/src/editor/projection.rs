use {
	super::{
		EditorEngine, EditorMode, EditorSelection, EditorTextLayerState, EditorViewState, EditorViewportMetrics,
		TextEdit,
		geometry::{cluster_rectangle, insert_selection_range, normal_selection_geometry, selection_rectangles},
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

impl EditorEngine {
	pub fn buffer(&self) -> Arc<Buffer> {
		// `EditorEngine` owns layout outright now, so hot-path reads no longer
		// bounce through an extra projection wrapper just to reach the buffer.
		self.layout.buffer()
	}

	pub fn sync_buffer_config(&mut self, font_system: &mut FontSystem, config: SceneConfig) -> bool {
		let Self { core, layout } = self;
		let changed = layout.sync_buffer_config(font_system, core.document.text(), config);
		if changed {
			self.refresh_view_state(None);
		}
		changed
	}

	pub fn sync_buffer_width(&mut self, font_system: &mut FontSystem, width: f32) -> bool {
		let changed = self.layout.sync_buffer_width(font_system, width);
		if changed {
			self.refresh_view_state_after_width_sync();
		}
		changed
	}

	pub fn view_state(&self) -> EditorViewState {
		self.layout.view_state()
	}

	pub fn shared_document_layout(&self) -> Arc<DocumentLayout> {
		self.layout.cached_document_layout_arc().unwrap_or_else(|| {
			// Seed the retained snapshot on first demand so later presentation reads
			// observe one derived layout instead of rebuilding independently.
			let layout = Arc::new(self.document_layout());
			self.layout.set_document_layout(Arc::clone(&layout));
			layout
		})
	}

	pub fn viewport_metrics(&self) -> EditorViewportMetrics {
		self.layout.viewport_metrics()
	}

	pub fn text_layer_state(&self) -> EditorTextLayerState {
		self.layout.text_layer_state()
	}

	#[cfg(test)]
	pub fn buffer_text(&self) -> String {
		self.layout.buffer_text()
	}

	pub fn document_layout(&self) -> DocumentLayout {
		let started = Instant::now();
		let document_layout = self.layout.document_layout(self.text());
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
		match self.mode() {
			EditorMode::Insert => self.layout.insert_cursor_block(self.text(), self.caret()),
			EditorMode::Normal => self.active_selection(layout).map(cluster_rectangle),
		}
	}

	pub fn refresh_view_state(&mut self, layout: Option<DocumentLayout>) {
		let started = Instant::now();
		let layout = Arc::new(layout.unwrap_or_else(|| self.document_layout()));
		let layout_ref = layout.as_ref();
		let layout_elapsed = started.elapsed();
		let (selection, selection_head) = self.selection_state();
		let tone = EditorOverlayTone::from(self.mode());
		let insert_cursor = if matches!(self.mode(), EditorMode::Insert) {
			selection_head.and_then(|head| self.layout.insert_cursor_rectangle(self.text(), head))
		} else {
			None
		};
		let viewport_target = self.active_viewport_target(layout_ref);
		let overlay_started = Instant::now();
		let overlays = self.build_overlays(layout_ref, self.selection(), insert_cursor, viewport_target, tone);
		let overlay_elapsed = overlay_started.elapsed();
		self.set_view_state(selection, selection_head, overlays, viewport_target);
		self.layout.set_document_layout(layout);
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
		match self.mode() {
			EditorMode::Insert => Self::insert_overlays(tone, insert_cursor, viewport_target),
			EditorMode::Normal => {
				let selection_rectangles = selection.map_or_else(
					|| Arc::from([]),
					|selection| selection_rectangles(layout, selection.range()),
				);
				Self::normal_overlays(tone, selection_rectangles.as_ref(), viewport_target)
			}
		}
	}

	pub fn buffer_hit(&self, point: Point) -> Option<cosmic_text::Cursor> {
		self.layout.hit(point)
	}

	pub fn apply_document_edit(&mut self, font_system: &mut FontSystem, edit: &TextEdit, structural: bool) -> TextEdit {
		if structural {
			// Hard line breaks can reshuffle buffer segmentation, so the retained
			// buffer has to be rebuilt from whole-document text after the edit.
			let inverse = self.core.document.apply_edit(edit);
			self.rebuild_buffer(font_system, edit);
			inverse
		} else {
			self.apply_buffer_edit(font_system, edit);
			self.core.document.apply_edit(edit)
		}
	}

	fn apply_buffer_edit(&mut self, font_system: &mut FontSystem, edit: &TextEdit) {
		let started = Instant::now();
		let Self { core, layout } = self;
		layout.apply_incremental_edit(font_system, core.document.text(), edit);
		self.log_buffer_edit("apply buffer edit", edit, started);
	}

	fn rebuild_buffer(&mut self, font_system: &mut FontSystem, edit: &TextEdit) {
		let started = Instant::now();
		let Self { core, layout } = self;
		layout.rebuild_buffer(font_system, core.document.text(), edit);
		self.log_buffer_edit("rebuild buffer", edit, started);
	}

	pub fn set_insert_head_fast(&mut self, head: usize) {
		// Incremental insert edits can derive the visible caret cell straight from
		// the retained buffer and avoid a full scene-layout rebuild.
		let selection =
			insert_selection_range(&self.buffer(), self.text(), head).map(|range| EditorSelection::new(range, head));
		self.core.session.enter_insert(selection);
	}

	pub fn refresh_insert_view_state_fast(&mut self) {
		let (selection, selection_head) = self.selection_state();
		let tone = EditorOverlayTone::from(self.mode());
		let insert_cursor = selection_head.and_then(|head| self.layout.insert_cursor_rectangle(self.text(), head));
		let viewport_target = selection_head.and_then(|head| self.layout.insert_cursor_block(self.text(), head));
		let overlays = Self::insert_overlays(tone, insert_cursor, viewport_target);
		self.set_view_state(selection, selection_head, overlays, viewport_target);
	}

	fn refresh_normal_view_state_fast(&mut self) {
		let (selection, selection_head) = self.selection_state();
		let tone = EditorOverlayTone::from(self.mode());
		let (selection_rectangles, viewport_target) = self.selection().map_or_else(
			|| (Arc::from([]), None),
			|selection| {
				normal_selection_geometry(
					&self.buffer(),
					self.text(),
					selection.range(),
					selection_head.unwrap_or_else(|| selection.range().start),
				)
			},
		);
		let overlays = Self::normal_overlays(tone, selection_rectangles.as_ref(), viewport_target);
		self.set_view_state(selection, selection_head, overlays, viewport_target);
	}

	fn set_view_state(
		&mut self, selection: Option<std::ops::Range<usize>>, selection_head: Option<usize>,
		overlays: Arc<[OverlayPrimitive]>, viewport_target: Option<LayoutRect>,
	) {
		self.layout.set_view_state(EditorViewState {
			mode: self.mode(),
			selection,
			selection_head,
			pointer_anchor: self.pointer_anchor(),
			overlays,
			viewport_target,
		});
	}

	fn selection_state(&self) -> (Option<std::ops::Range<usize>>, Option<usize>) {
		let selection = self.selection();
		(
			selection.map(|selection| selection.range().clone()),
			selection.map(EditorSelection::head),
		)
	}

	fn insert_overlays(
		tone: EditorOverlayTone, insert_cursor: Option<LayoutRect>, viewport_target: Option<LayoutRect>,
	) -> Arc<[OverlayPrimitive]> {
		viewport_target
			.into_iter()
			.map(|insert_block| {
				OverlayPrimitive::scene_rect(
					insert_block,
					OverlayRectKind::EditorInsertBlock(tone),
					OverlayLayer::UnderText,
				)
			})
			.chain(insert_cursor.into_iter().map(|caret| {
				OverlayPrimitive::scene_rect(caret, OverlayRectKind::EditorCaret(tone), OverlayLayer::UnderText)
			}))
			.collect()
	}

	fn normal_overlays(
		tone: EditorOverlayTone, selection_rectangles: &[LayoutRect], viewport_target: Option<LayoutRect>,
	) -> Arc<[OverlayPrimitive]> {
		selection_rectangles
			.iter()
			.copied()
			.map(|rect| OverlayPrimitive::scene_rect(rect, OverlayRectKind::EditorSelection, OverlayLayer::UnderText))
			.chain(viewport_target.into_iter().map(|active| {
				OverlayPrimitive::scene_rect(active, OverlayRectKind::EditorActive(tone), OverlayLayer::UnderText)
			}))
			.collect()
	}

	fn log_buffer_edit(&self, label: &'static str, edit: &TextEdit, started: Instant) {
		let total_ms = duration_ms(started.elapsed());
		if total_ms >= 8.0 {
			debug!(
				text_bytes = self.core.document.len(),
				total_ms,
				inserted_bytes = edit.inserted.len(),
				range_start = edit.range.start,
				range_end = edit.range.end,
				"{label}"
			);
		} else {
			trace!(
				text_bytes = self.core.document.len(),
				total_ms,
				inserted_bytes = edit.inserted.len(),
				range_start = edit.range.start,
				range_end = edit.range.end,
				"{label}"
			);
		}
	}
}
