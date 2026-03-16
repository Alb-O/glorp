mod document;
mod editing;
mod geometry;
mod history;
mod layout;
mod layout_state;
mod navigation;
mod reducer;
mod selection;
mod session;
mod text;

#[cfg(test)]
mod tests;

use {
	self::{
		document::DocumentState,
		geometry::{cluster_rectangle, insert_selection_range, normal_selection_geometry, selection_rectangles},
		history::{EditorSnapshot, HistoryEntry},
		layout::BufferClusterInfo,
		layout_state::EditorLayout,
		reducer::apply_intent,
		session::EditorSession,
	},
	crate::{
		overlay::{EditorOverlayTone, LayoutRect, OverlayLayer, OverlayPrimitive, OverlayRectKind},
		scene::SceneConfig,
		telemetry::duration_ms,
		types::WrapChoice,
	},
	cosmic_text::{Buffer, FontSystem},
	iced::Point,
	std::{
		fmt::{self, Display},
		ops::Range,
		sync::Arc,
		time::Instant,
	},
	tracing::{debug, trace, trace_span, warn},
};

pub(crate) use self::layout::BufferLayoutSnapshot;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum EditorMode {
	Normal,
	Insert,
}

impl Display for EditorMode {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Normal => f.write_str("Normal"),
			Self::Insert => f.write_str("Insert"),
		}
	}
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EditorViewState {
	pub(crate) mode: EditorMode,
	pub(crate) selection: Option<Range<usize>>,
	pub(crate) selection_head: Option<usize>,
	pub(crate) pointer_anchor: Option<usize>,
	pub(crate) overlays: Arc<[OverlayPrimitive]>,
	pub(crate) viewport_target: Option<LayoutRect>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct EditorSelection {
	range: Range<usize>,
	head: usize,
}

impl EditorSelection {
	fn new(range: Range<usize>, head: usize) -> Self {
		Self { range, head }
	}

	fn clamped(&self, document_len: usize) -> Self {
		let start = self.range.start.min(document_len);
		let end = self.range.end.min(document_len).max(start);
		let head = self.head.min(document_len);
		Self {
			range: start..end,
			head,
		}
	}

	fn range(&self) -> &Range<usize> {
		&self.range
	}

	fn range_cloned(&self) -> Range<usize> {
		self.range.clone()
	}

	fn head(&self) -> usize {
		self.head
	}
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum EditorIntent {
	Pointer(EditorPointerIntent),
	Motion(EditorMotion),
	Mode(EditorModeIntent),
	Edit(EditorEditIntent),
	History(EditorHistoryIntent),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum EditorPointerIntent {
	Begin { position: Point, select_word: bool },
	Drag(Point),
	End,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EditorMotion {
	Left,
	Right,
	Up,
	Down,
	LineStart,
	LineEnd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EditorModeIntent {
	EnterInsertBefore,
	EnterInsertAfter,
	ExitInsert,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum EditorEditIntent {
	Backspace,
	DeleteForward,
	DeleteSelection,
	InsertText(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EditorHistoryIntent {
	Undo,
	Redo,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TextEdit {
	pub(crate) range: Range<usize>,
	pub(crate) inserted: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct EditorOutcome {
	pub(crate) view_changed: bool,
	pub(crate) selection_changed: bool,
	pub(crate) mode_changed: bool,
	pub(crate) viewport_target: Option<LayoutRect>,
	pub(crate) text_edit: Option<TextEdit>,
}

#[derive(Debug, Clone, Default)]
struct ApplyResult {
	text_edit: Option<TextEdit>,
	layout: Option<BufferLayoutSnapshot>,
	view_refreshed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct EditorViewportMetrics {
	pub(crate) wrapping: WrapChoice,
	pub(crate) measured_width: f32,
	pub(crate) measured_height: f32,
}

#[derive(Debug, Clone)]
pub(crate) struct EditorTextLayerState {
	pub(crate) buffer: std::sync::Weak<cosmic_text::Buffer>,
	pub(crate) measured_height: f32,
}

#[derive(Debug, Clone)]
pub(crate) struct EditorState {
	document: DocumentState,
	session: EditorSession,
}

#[derive(Debug, Clone)]
pub(crate) struct EditorLayoutModel {
	layout: EditorLayout,
}

#[derive(Debug, Clone)]
pub(crate) struct EditorEngine {
	state: EditorState,
	layout_model: EditorLayoutModel,
}

impl EditorOutcome {
	fn from_apply_result(previous_view: &EditorViewState, next_view: &EditorViewState, result: ApplyResult) -> Self {
		Self {
			view_changed: previous_view != next_view,
			selection_changed: previous_view.selection != next_view.selection
				|| previous_view.selection_head != next_view.selection_head,
			mode_changed: previous_view.mode != next_view.mode,
			viewport_target: next_view.viewport_target,
			text_edit: result.text_edit,
		}
	}

	pub(crate) fn document_changed(&self) -> bool {
		self.text_edit.is_some()
	}

	pub(crate) fn requires_scene_rebuild(&self) -> bool {
		self.document_changed()
	}
}

impl EditorViewState {
	pub(crate) fn overlay_count(&self, kind: OverlayRectKind) -> usize {
		self.overlays.iter().filter(|primitive| primitive.kind == kind).count()
	}
}

impl EditorState {
	fn new(text: impl Into<String>) -> Self {
		Self {
			document: DocumentState::new(text),
			session: EditorSession::new(),
		}
	}
}

impl EditorLayoutModel {
	fn new(font_system: &mut FontSystem, text: &str, config: SceneConfig) -> Self {
		Self {
			layout: EditorLayout::new(font_system, text, config),
		}
	}
}

impl EditorEngine {
	pub(crate) fn new(font_system: &mut FontSystem, text: impl Into<String>, config: SceneConfig) -> Self {
		let state = EditorState::new(text);
		let mut editor = Self {
			layout_model: EditorLayoutModel::new(font_system, state.document.text(), config),
			state,
		};
		editor.reset_normal_selection();
		editor.refresh_view_state(None);
		editor
	}

	pub(crate) fn text(&self) -> &str {
		self.state.document.text()
	}

	pub(crate) fn mode(&self) -> EditorMode {
		self.state.session.mode()
	}

	pub(crate) fn buffer(&self) -> Arc<Buffer> {
		self.layout_model.layout.buffer()
	}

	pub(crate) fn history_depths(&self) -> (usize, usize) {
		self.state.document.history_depths()
	}

	pub(crate) fn sync_buffer_config(&mut self, font_system: &mut FontSystem, config: SceneConfig) -> bool {
		let Self { state, layout_model } = self;
		if layout_model
			.layout
			.sync_buffer_config(font_system, state.document.text(), config)
		{
			self.refresh_view_state(None);
			return true;
		}

		false
	}

	pub(crate) fn sync_buffer_width(&mut self, font_system: &mut FontSystem, width: f32) {
		if self.layout_model.layout.sync_buffer_width(font_system, width) {
			self.refresh_view_state_after_width_sync();
		}
	}

	pub(crate) fn reset(&mut self, font_system: &mut FontSystem, text: impl Into<String>, config: SceneConfig) {
		self.state.document.reset(text);
		self.state.session = EditorSession::new();
		let Self { state, layout_model } = self;
		layout_model.layout.reset(font_system, state.document.text(), config);
		self.reset_normal_selection();
		self.refresh_view_state(None);
	}

	pub(crate) fn view_state(&self) -> EditorViewState {
		self.layout_model.layout.view_state()
	}

	pub(crate) fn view_state_ref(&self) -> &EditorViewState {
		self.layout_model.layout.view_state_ref()
	}

	pub(crate) fn with_cached_layout_snapshot<T>(&self, f: impl FnOnce(Option<&BufferLayoutSnapshot>) -> T) -> T {
		self.layout_model.layout.cached_snapshot(f)
	}

	pub(crate) fn viewport_metrics(&self) -> EditorViewportMetrics {
		self.layout_model.layout.viewport_metrics()
	}

	pub(crate) fn text_layer_state(&self) -> EditorTextLayerState {
		self.layout_model.layout.text_layer_state()
	}

	pub(crate) fn apply(&mut self, font_system: &mut FontSystem, intent: EditorIntent) -> EditorOutcome {
		let _span = trace_span!("editor.apply", intent = ?intent).entered();
		let previous_view = self.layout_model.layout.view_state();
		let apply_started = Instant::now();
		let ApplyResult {
			text_edit,
			layout,
			view_refreshed,
		} = apply_intent(self, font_system, intent);
		let reducer_elapsed = apply_started.elapsed();
		let refresh_started = Instant::now();
		if !view_refreshed {
			self.refresh_view_state(layout);
		}
		let refresh_elapsed = refresh_started.elapsed();
		let total_elapsed = apply_started.elapsed();
		let total_ms = duration_ms(total_elapsed);
		if total_ms >= 16.7 {
			warn!(
				reducer_ms = duration_ms(reducer_elapsed),
				refresh_ms = duration_ms(refresh_elapsed),
				total_ms,
				text_bytes = self.text().len(),
				"editor apply over frame budget"
			);
		} else if total_ms >= 8.0 {
			debug!(
				reducer_ms = duration_ms(reducer_elapsed),
				refresh_ms = duration_ms(refresh_elapsed),
				total_ms,
				text_bytes = self.text().len(),
				"editor apply over warning threshold"
			);
		} else {
			trace!(
				reducer_ms = duration_ms(reducer_elapsed),
				refresh_ms = duration_ms(refresh_elapsed),
				total_ms,
				text_bytes = self.text().len(),
				"editor apply"
			);
		}
		EditorOutcome::from_apply_result(
			&previous_view,
			self.layout_model.layout.view_state_ref(),
			ApplyResult {
				text_edit,
				layout: None,
				view_refreshed: false,
			},
		)
	}

	#[cfg(test)]
	pub(crate) fn buffer_text(&self) -> String {
		self.layout_model.layout.buffer_text()
	}

	fn layout_snapshot(&self) -> BufferLayoutSnapshot {
		let started = Instant::now();
		let snapshot = self.layout_model.layout.snapshot(self.text());
		let elapsed_ms = duration_ms(started.elapsed());
		if elapsed_ms >= 8.0 {
			debug!(
				duration_ms = elapsed_ms,
				clusters = snapshot.clusters().len(),
				text_bytes = self.text().len(),
				"layout snapshot"
			);
		} else {
			trace!(
				duration_ms = elapsed_ms,
				clusters = snapshot.clusters().len(),
				text_bytes = self.text().len(),
				"layout snapshot"
			);
		}
		snapshot
	}

	fn reset_normal_selection(&mut self) {
		if let Some(selection) = self
			.layout_snapshot()
			.cluster(0)
			.map(|cluster| cluster.byte_range.clone())
		{
			let head = selection.start;
			self.state
				.session
				.set_normal_selection(EditorSelection::new(selection, head), None, Some(head));
		} else {
			self.set_selection(None);
			self.clear_pointer_anchor();
		}
	}

	fn select_cluster(&mut self, layout: &BufferLayoutSnapshot, cluster_index: usize) {
		let Some(cluster) = layout.cluster(cluster_index) else {
			return;
		};

		self.state.session.set_normal_selection(
			EditorSelection::new(cluster.byte_range.clone(), cluster.byte_range.start),
			Some(cluster.center_x()),
			Some(cluster.byte_range.start),
		);
	}

	fn active_selection_index(&self, layout: &BufferLayoutSnapshot) -> Option<usize> {
		self.selection()?;

		layout
			.cluster_at_or_after(self.caret())
			.or_else(|| layout.cluster_before(self.caret().saturating_add(1)))
	}

	fn active_selection<'a>(&self, layout: &'a BufferLayoutSnapshot) -> Option<&'a BufferClusterInfo> {
		self.active_selection_index(layout)
			.and_then(|index| layout.cluster(index))
	}

	fn history_snapshot(&self) -> EditorSnapshot {
		self.state.session.history_snapshot()
	}

	fn restore_snapshot(&mut self, snapshot: &EditorSnapshot) {
		self.state.session.restore_snapshot(snapshot, self.state.document.len());
	}

	fn apply_document_edit(&mut self, font_system: &mut FontSystem, edit: &TextEdit) -> TextEdit {
		self.apply_buffer_edit(font_system, edit);
		self.state.document.apply_edit(edit)
	}

	fn record_history(&mut self, forward: TextEdit, inverse: TextEdit, before: EditorSnapshot) {
		self.state.document.record_history(HistoryEntry {
			forward,
			inverse,
			before,
			after: self.history_snapshot(),
		});
	}

	fn active_viewport_target(&self, layout: &BufferLayoutSnapshot) -> Option<LayoutRect> {
		if matches!(self.mode(), EditorMode::Insert) {
			return self.layout_model.layout.insert_cursor_block(self.text(), self.caret());
		}

		self.active_selection(layout).map(cluster_rectangle)
	}

	fn refresh_view_state(&mut self, layout: Option<BufferLayoutSnapshot>) {
		let started = Instant::now();
		let layout = Arc::new(layout.unwrap_or_else(|| self.layout_snapshot()));
		let layout_ref = layout.as_ref();
		let layout_elapsed = started.elapsed();
		let selection = self.selection().cloned();
		let selection_head = selection.as_ref().map(EditorSelection::head);
		let tone = EditorOverlayTone::from(self.mode());
		let insert_cursor = if matches!(self.mode(), EditorMode::Insert) {
			selection_head.and_then(|head| self.layout_model.layout.insert_cursor_rectangle(self.text(), head))
		} else {
			None
		};
		let viewport_target = self.active_viewport_target(layout_ref);
		let overlay_started = Instant::now();
		let overlays = self.build_overlays(layout_ref, selection.as_ref(), insert_cursor, viewport_target, tone);
		let overlay_elapsed = overlay_started.elapsed();
		self.layout_model.layout.set_view_state(EditorViewState {
			mode: self.mode(),
			selection: selection.as_ref().map(EditorSelection::range_cloned),
			selection_head,
			pointer_anchor: self.pointer_anchor(),
			overlays,
			viewport_target,
		});
		self.layout_model.layout.set_snapshot(layout);
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
		&self, layout: &BufferLayoutSnapshot, selection: Option<&EditorSelection>, insert_cursor: Option<LayoutRect>,
		viewport_target: Option<LayoutRect>, tone: EditorOverlayTone,
	) -> Arc<[OverlayPrimitive]> {
		let mut overlays = Vec::new();

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

	fn selection(&self) -> Option<&EditorSelection> {
		self.state.session.selection()
	}

	fn selection_range(&self) -> Option<Range<usize>> {
		self.selection().map(EditorSelection::range_cloned)
	}

	fn set_selection(&mut self, selection: Option<EditorSelection>) {
		self.state.session.set_selection(selection);
	}

	fn set_mode(&mut self, mode: EditorMode) {
		self.state.session.set_mode(mode);
	}

	fn caret(&self) -> usize {
		self.state.session.caret()
	}

	fn preferred_x(&self) -> Option<f32> {
		self.state.session.preferred_x()
	}

	fn set_preferred_x(&mut self, preferred_x: Option<f32>) {
		self.state.session.set_preferred_x(preferred_x);
	}

	fn pointer_anchor(&self) -> Option<usize> {
		self.state.session.pointer_anchor()
	}

	fn set_pointer_anchor(&mut self, pointer_anchor: Option<usize>) {
		self.state.session.set_pointer_anchor(pointer_anchor);
	}

	fn clear_pointer_anchor(&mut self) {
		self.set_pointer_anchor(None);
	}

	fn enter_insert_at(&mut self, caret: usize) {
		let layout = self.layout_snapshot();
		self.enter_insert_with_layout(&layout, caret);
	}

	fn enter_insert_with_layout(&mut self, layout: &BufferLayoutSnapshot, caret: usize) {
		self.set_insert_head(layout, caret);
	}

	fn buffer_hit(&self, point: Point) -> Option<cosmic_text::Cursor> {
		self.layout_model.layout.hit(point)
	}

	fn apply_buffer_edit(&mut self, font_system: &mut FontSystem, edit: &TextEdit) {
		let started = Instant::now();
		let Self { state, layout_model } = self;
		let apply_started = Instant::now();
		layout_model.layout.apply_edit(font_system, state.document.text(), edit);
		let apply_elapsed = apply_started.elapsed();
		let total_ms = duration_ms(started.elapsed());
		if total_ms >= 8.0 {
			debug!(
				text_bytes = state.document.len(),
				layout_apply_ms = duration_ms(apply_elapsed),
				total_ms,
				inserted_bytes = edit.inserted.len(),
				range_start = edit.range.start,
				range_end = edit.range.end,
				"apply buffer edit"
			);
		} else {
			trace!(
				text_bytes = state.document.len(),
				layout_apply_ms = duration_ms(apply_elapsed),
				total_ms,
				inserted_bytes = edit.inserted.len(),
				range_start = edit.range.start,
				range_end = edit.range.end,
				"apply buffer edit"
			);
		}
	}

	fn insert_selection(layout: &BufferLayoutSnapshot, head: usize) -> Option<EditorSelection> {
		layout
			.cluster_at_insert_head(head)
			.and_then(|index| layout.cluster(index))
			.map(|cluster| EditorSelection::new(cluster.byte_range.clone(), head))
	}

	fn set_insert_head(&mut self, layout: &BufferLayoutSnapshot, head: usize) {
		self.state.session.enter_insert(Self::insert_selection(layout, head));
	}

	fn set_insert_head_fast(&mut self, head: usize) {
		let selection =
			insert_selection_range(&self.buffer(), self.text(), head).map(|range| EditorSelection::new(range, head));
		self.state.session.enter_insert(selection);
	}

	fn refresh_insert_view_state_fast(&mut self) {
		let selection = self.selection().cloned();
		let selection_head = selection.as_ref().map(EditorSelection::head);
		let tone = EditorOverlayTone::from(self.mode());
		let insert_cursor =
			selection_head.and_then(|head| self.layout_model.layout.insert_cursor_rectangle(self.text(), head));
		let viewport_target =
			selection_head.and_then(|head| self.layout_model.layout.insert_cursor_block(self.text(), head));
		let mut overlays = Vec::new();

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

		self.layout_model.layout.set_view_state(EditorViewState {
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
		let mut overlays = Vec::new();

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

		self.layout_model.layout.set_view_state(EditorViewState {
			mode: self.mode(),
			selection: selection.as_ref().map(EditorSelection::range_cloned),
			selection_head,
			pointer_anchor: self.pointer_anchor(),
			overlays: overlays.into(),
			viewport_target,
		});
	}
}
