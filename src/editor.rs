use cosmic_text::{Buffer, Cursor, Edit as _, Editor as CosmicEditor, FontSystem};
use iced::Point;

use std::fmt::{self, Display};
use std::ops::Range;
use std::sync::Arc;

use crate::scene::{SceneConfig, build_buffer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone)]
pub(crate) struct EditorViewState {
	pub(crate) mode: EditorMode,
	pub(crate) selection: Option<Range<usize>>,
	pub(crate) caret: usize,
	pub(crate) selection_rectangles: Arc<[EditorSelectionRect]>,
	pub(crate) caret_geometry: EditorCaretGeometry,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct EditorSelectionRect {
	pub(crate) x: f32,
	pub(crate) y: f32,
	pub(crate) width: f32,
	pub(crate) height: f32,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct EditorCaretGeometry {
	pub(crate) x: f32,
	pub(crate) y: f32,
	pub(crate) height: f32,
}

#[derive(Debug, Clone)]
pub(crate) struct EditorBuffer {
	text: String,
	buffer: Arc<Buffer>,
	config: SceneConfig,
	mode: EditorMode,
	selection: Option<Range<usize>>,
	caret: usize,
	preferred_x: Option<f32>,
	pointer_anchor: Option<usize>,
	view_state: EditorViewState,
}

#[derive(Debug, Clone)]
pub(crate) enum EditorCommand {
	BeginPointerSelection { position: Point, select_word: bool },
	DragPointerSelection(Point),
	EndPointerSelection,
	MoveLeft,
	MoveRight,
	MoveUp,
	MoveDown,
	MoveLineStart,
	MoveLineEnd,
	EnterInsertBefore,
	EnterInsertAfter,
	ExitInsert,
	Backspace,
	DeleteForward,
	DeleteSelection,
	InsertText(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TextEdit {
	pub(crate) range: Range<usize>,
	pub(crate) inserted: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ApplyResult {
	pub(crate) changed: bool,
	pub(crate) text_edit: Option<TextEdit>,
}

impl EditorBuffer {
	pub(crate) fn new(font_system: &mut FontSystem, text: impl Into<String>, config: SceneConfig) -> Self {
		let text = text.into();
		let mut editor = Self {
			buffer: Arc::new(build_buffer(font_system, &text, config)),
			text,
			config,
			mode: EditorMode::Normal,
			selection: None,
			caret: 0,
			preferred_x: None,
			pointer_anchor: None,
			view_state: EditorViewState {
				mode: EditorMode::Normal,
				selection: None,
				caret: 0,
				selection_rectangles: Arc::from([]),
				caret_geometry: EditorCaretGeometry {
					x: 0.0,
					y: 0.0,
					height: config.line_height.max(1.0),
				},
			},
		};
		editor.reset_normal_selection();
		editor.refresh_view_state();
		editor
	}

	pub(crate) fn text(&self) -> &str {
		&self.text
	}

	pub(crate) fn mode(&self) -> EditorMode {
		self.mode
	}

	pub(crate) fn buffer(&self) -> Arc<Buffer> {
		self.buffer.clone()
	}

	pub(crate) fn sync_buffer_config(&mut self, font_system: &mut FontSystem, config: SceneConfig) {
		if self.config == config {
			return;
		}

		if self.width_only_config_change(config) {
			self.resize_buffer(font_system, config.max_width);
			self.config = config;
			self.refresh_view_state();
			return;
		}

		self.config = config;
		self.buffer = Arc::new(build_buffer(font_system, &self.text, config));
		self.refresh_view_state();
	}

	pub(crate) fn sync_buffer_width(&mut self, font_system: &mut FontSystem, width: f32) {
		if (self.config.max_width - width).abs() < 0.5 {
			return;
		}

		self.resize_buffer(font_system, width);
		self.config.max_width = width;
		self.refresh_view_state();
	}

	pub(crate) fn reset(&mut self, font_system: &mut FontSystem, text: impl Into<String>, config: SceneConfig) {
		self.text = text.into();
		self.config = config;
		self.buffer = Arc::new(build_buffer(font_system, &self.text, config));
		self.mode = EditorMode::Normal;
		self.selection = None;
		self.caret = 0;
		self.preferred_x = None;
		self.pointer_anchor = None;
		self.reset_normal_selection();
		self.refresh_view_state();
	}

	pub(crate) fn view_state(&self) -> EditorViewState {
		self.view_state.clone()
	}

	pub(crate) fn apply(&mut self, font_system: &mut FontSystem, command: EditorCommand) -> ApplyResult {
		let result = match command {
			EditorCommand::BeginPointerSelection { position, select_word } => {
				let layout = self.layout_snapshot();
				if select_word {
					self.select_word_at(&layout, position);
				} else if let Some(cluster_index) = self.pointer_cluster_index(&layout, position) {
					self.pointer_anchor = layout.cluster(cluster_index).map(|cluster| cluster.byte_range.start);
					self.select_cluster(&layout, cluster_index);
				} else if self.text.is_empty() {
					self.mode = EditorMode::Insert;
					self.selection = None;
					self.caret = 0;
					self.preferred_x = None;
					self.pointer_anchor = None;
				}
				ApplyResult {
					changed: false,
					text_edit: None,
				}
			}
			EditorCommand::DragPointerSelection(position) => {
				let layout = self.layout_snapshot();
				self.extend_pointer_selection(&layout, position);
				ApplyResult {
					changed: false,
					text_edit: None,
				}
			}
			EditorCommand::EndPointerSelection => {
				self.pointer_anchor = None;
				ApplyResult {
					changed: false,
					text_edit: None,
				}
			}
			EditorCommand::MoveLeft => {
				let layout = self.layout_snapshot();
				self.move_left(&layout);
				ApplyResult {
					changed: false,
					text_edit: None,
				}
			}
			EditorCommand::MoveRight => {
				let layout = self.layout_snapshot();
				self.move_right(&layout);
				ApplyResult {
					changed: false,
					text_edit: None,
				}
			}
			EditorCommand::MoveUp => {
				let layout = self.layout_snapshot();
				self.move_vertical(&layout, -1);
				ApplyResult {
					changed: false,
					text_edit: None,
				}
			}
			EditorCommand::MoveDown => {
				let layout = self.layout_snapshot();
				self.move_vertical(&layout, 1);
				ApplyResult {
					changed: false,
					text_edit: None,
				}
			}
			EditorCommand::MoveLineStart => {
				let layout = self.layout_snapshot();
				self.move_line_edge(&layout, true);
				ApplyResult {
					changed: false,
					text_edit: None,
				}
			}
			EditorCommand::MoveLineEnd => {
				let layout = self.layout_snapshot();
				self.move_line_edge(&layout, false);
				ApplyResult {
					changed: false,
					text_edit: None,
				}
			}
			EditorCommand::EnterInsertBefore => {
				self.mode = EditorMode::Insert;
				self.caret = self.selection.as_ref().map(|selection| selection.start).unwrap_or(0);
				self.preferred_x = None;
				self.pointer_anchor = None;
				ApplyResult {
					changed: false,
					text_edit: None,
				}
			}
			EditorCommand::EnterInsertAfter => {
				self.mode = EditorMode::Insert;
				self.caret = self
					.selection
					.as_ref()
					.map(|selection| selection.end)
					.unwrap_or(self.text.len());
				self.preferred_x = None;
				self.pointer_anchor = None;
				ApplyResult {
					changed: false,
					text_edit: None,
				}
			}
			EditorCommand::ExitInsert => {
				self.exit_insert();
				ApplyResult {
					changed: false,
					text_edit: None,
				}
			}
			EditorCommand::Backspace => self.backspace(font_system),
			EditorCommand::DeleteForward => self.delete_forward(font_system),
			EditorCommand::DeleteSelection => self.delete_selection(font_system),
			EditorCommand::InsertText(text) => self.insert_text(font_system, text),
		};
		self.refresh_view_state();
		result
	}

	pub(crate) fn selection_details(&self) -> String {
		match self.mode {
			EditorMode::Normal => {
				let Some(selection) = &self.selection else {
					return format!("  mode: {}\n  selection: none", self.mode);
				};

				format!(
					"  mode: {}\n  bytes: {:?}\n  text: {}\n  rects: {}\n  active byte: {}\n  anchor byte: {}",
					self.mode,
					selection,
					self.preview_range(selection),
					self.view_state.selection_rectangles.len(),
					self.caret,
					self.pointer_anchor.unwrap_or(selection.start),
				)
			}
			EditorMode::Insert => format!(
				"  mode: {}\n  caret byte: {}\n  caret x/y: {:.1}, {:.1}\n  caret height: {:.1}",
				self.mode,
				self.caret,
				self.view_state.caret_geometry.x,
				self.view_state.caret_geometry.y,
				self.view_state.caret_geometry.height,
			),
		}
	}

	fn move_left(&mut self, layout: &BufferLayoutSnapshot) {
		self.pointer_anchor = None;
		match self.mode {
			EditorMode::Normal => {
				let Some(current) = self.active_selection_index(layout) else {
					return;
				};

				if let Some(previous) = current.checked_sub(1) {
					self.select_cluster(layout, previous);
				}
			}
			EditorMode::Insert => {
				self.caret = previous_char_boundary(&self.text, self.caret).unwrap_or(0);
				self.preferred_x = None;
			}
		}
	}

	fn move_right(&mut self, layout: &BufferLayoutSnapshot) {
		self.pointer_anchor = None;
		match self.mode {
			EditorMode::Normal => {
				let Some(current) = self.active_selection_index(layout) else {
					return;
				};

				if current + 1 < layout.clusters().len() {
					self.select_cluster(layout, current + 1);
				}
			}
			EditorMode::Insert => {
				self.caret = next_char_boundary(&self.text, self.caret).unwrap_or(self.text.len());
				self.preferred_x = None;
			}
		}
	}

	fn move_vertical(&mut self, layout: &BufferLayoutSnapshot, direction: isize) {
		self.pointer_anchor = None;
		match self.mode {
			EditorMode::Normal => {
				let Some(current) = self.active_selection(layout) else {
					return;
				};
				let preferred_x = self.preferred_x.unwrap_or_else(|| current.center_x());
				let Some(target) = layout.nearest_cluster_on_adjacent_run(current.run_index, preferred_x, direction)
				else {
					return;
				};
				self.select_cluster(layout, target);
				self.preferred_x = Some(preferred_x);
			}
			EditorMode::Insert => {
				let caret = layout.caret_metrics(self.caret, self.config.line_height);
				let preferred_x = self.preferred_x.unwrap_or(caret.x);
				let Some(target) = layout.nearest_cluster_on_adjacent_run(caret.run_index, preferred_x, direction)
				else {
					return;
				};
				let cluster = &layout.clusters()[target];
				self.caret = if preferred_x > cluster.center_x() {
					cluster.byte_range.end
				} else {
					cluster.byte_range.start
				};
				self.preferred_x = Some(preferred_x);
			}
		}
	}

	fn move_line_edge(&mut self, layout: &BufferLayoutSnapshot, to_start: bool) {
		self.pointer_anchor = None;
		match self.mode {
			EditorMode::Normal => {
				let Some(current) = self.active_selection(layout) else {
					return;
				};
				let target = if to_start {
					layout.first_cluster_in_run(current.run_index)
				} else {
					layout.last_cluster_in_run(current.run_index)
				};

				if let Some(target) = target {
					self.select_cluster(layout, target);
				}
			}
			EditorMode::Insert => {
				let caret = layout.caret_metrics(self.caret, self.config.line_height);
				let target = if to_start {
					layout
						.first_cluster_in_run(caret.run_index)
						.map(|index| layout.clusters()[index].byte_range.start)
						.unwrap_or(self.caret)
				} else {
					layout
						.last_cluster_in_run(caret.run_index)
						.map(|index| layout.clusters()[index].byte_range.end)
						.unwrap_or(self.caret)
				};

				self.caret = target;
				self.preferred_x = None;
			}
		}
	}

	fn exit_insert(&mut self) {
		let layout = self.layout_snapshot();
		self.mode = EditorMode::Normal;
		self.preferred_x = None;
		self.pointer_anchor = None;

		self.selection = layout
			.cluster_before(self.caret)
			.or_else(|| layout.cluster_at_or_after(self.caret))
			.and_then(|index| layout.cluster(index))
			.map(|cluster| cluster.byte_range.clone());
	}

	fn delete_selection(&mut self, font_system: &mut FontSystem) -> ApplyResult {
		let Some(selection) = self.selection.clone() else {
			return ApplyResult {
				changed: false,
				text_edit: None,
			};
		};

		let text_edit = TextEdit {
			range: selection.clone(),
			inserted: String::new(),
		};
		self.apply_buffer_edit(font_system, &text_edit);
		self.text.replace_range(selection.clone(), "");
		self.mode = EditorMode::Normal;
		self.pointer_anchor = None;
		let next_layout = self.layout_snapshot();
		self.selection = next_layout
			.cluster_at_or_after(selection.start)
			.or_else(|| next_layout.cluster_before(selection.start))
			.and_then(|index| next_layout.cluster(index))
			.map(|cluster| cluster.byte_range.clone());
		self.caret = clamp_char_boundary(&self.text, selection.start);
		self.preferred_x = None;
		ApplyResult {
			changed: true,
			text_edit: Some(text_edit),
		}
	}

	fn backspace(&mut self, font_system: &mut FontSystem) -> ApplyResult {
		match self.mode {
			EditorMode::Normal => self.delete_selection(font_system),
			EditorMode::Insert => {
				let Some(previous) = previous_char_boundary(&self.text, self.caret) else {
					return ApplyResult {
						changed: false,
						text_edit: None,
					};
				};

				let range = previous..self.caret;
				let text_edit = TextEdit {
					range: range.clone(),
					inserted: String::new(),
				};
				self.apply_buffer_edit(font_system, &text_edit);
				self.text.replace_range(previous..self.caret, "");
				self.caret = previous;
				self.preferred_x = None;
				self.pointer_anchor = None;
				ApplyResult {
					changed: true,
					text_edit: Some(text_edit),
				}
			}
		}
	}

	fn delete_forward(&mut self, font_system: &mut FontSystem) -> ApplyResult {
		match self.mode {
			EditorMode::Normal => self.delete_selection(font_system),
			EditorMode::Insert => {
				let Some(next) = next_char_boundary(&self.text, self.caret) else {
					return ApplyResult {
						changed: false,
						text_edit: None,
					};
				};

				let range = self.caret..next;
				let text_edit = TextEdit {
					range: range.clone(),
					inserted: String::new(),
				};
				self.apply_buffer_edit(font_system, &text_edit);
				self.text.replace_range(self.caret..next, "");
				self.preferred_x = None;
				self.pointer_anchor = None;
				ApplyResult {
					changed: true,
					text_edit: Some(text_edit),
				}
			}
		}
	}

	fn insert_text(&mut self, font_system: &mut FontSystem, text: String) -> ApplyResult {
		if text.is_empty() {
			return ApplyResult {
				changed: false,
				text_edit: None,
			};
		}

		if !matches!(self.mode, EditorMode::Insert) {
			self.mode = EditorMode::Insert;
		}

		self.caret = clamp_char_boundary(&self.text, self.caret);
		let range = self.caret..self.caret;
		let text_edit = TextEdit {
			range: range.clone(),
			inserted: text,
		};
		self.apply_buffer_edit(font_system, &text_edit);
		self.text.insert_str(self.caret, &text_edit.inserted);
		self.caret += text_edit.inserted.len();
		self.preferred_x = None;
		self.pointer_anchor = None;
		ApplyResult {
			changed: true,
			text_edit: Some(text_edit),
		}
	}

	pub(crate) fn buffer_text(&self) -> String {
		let mut text = String::new();
		for line in &self.buffer.lines {
			text.push_str(line.text());
			text.push_str(line.ending().as_str());
		}
		text
	}

	fn layout_snapshot(&self) -> BufferLayoutSnapshot {
		BufferLayoutSnapshot::new(&self.buffer, &self.text)
	}

	fn reset_normal_selection(&mut self) {
		self.selection = self
			.layout_snapshot()
			.cluster(0)
			.map(|cluster| cluster.byte_range.clone());

		if let Some(selection) = &self.selection {
			self.caret = selection.start;
			self.pointer_anchor = Some(selection.start);
		}
	}

	fn select_cluster(&mut self, layout: &BufferLayoutSnapshot, cluster_index: usize) {
		let Some(cluster) = layout.cluster(cluster_index) else {
			return;
		};

		self.mode = EditorMode::Normal;
		self.selection = Some(cluster.byte_range.clone());
		self.caret = cluster.byte_range.start;
		self.preferred_x = Some(cluster.center_x());
		self.pointer_anchor = Some(cluster.byte_range.start);
	}

	fn active_selection_index(&self, layout: &BufferLayoutSnapshot) -> Option<usize> {
		if self.selection.is_none() {
			return None;
		}

		layout
			.cluster_at_or_after(self.caret)
			.or_else(|| layout.cluster_before(self.caret.saturating_add(1)))
	}

	fn active_selection<'a>(&self, layout: &'a BufferLayoutSnapshot) -> Option<&'a BufferClusterInfo> {
		self.active_selection_index(layout)
			.and_then(|index| layout.cluster(index))
	}

	fn pointer_cluster_index(&self, layout: &BufferLayoutSnapshot, point: Point) -> Option<usize> {
		self.buffer
			.hit(point.x, point.y)
			.and_then(|cursor| layout.cluster_index_for_cursor(cursor))
			.or_else(|| {
				layout
					.nearest_cluster_at(point.y, point.x)
					.or_else(|| (!layout.clusters().is_empty()).then_some(0))
			})
	}

	fn extend_pointer_selection(&mut self, layout: &BufferLayoutSnapshot, position: Point) {
		let Some(anchor_byte) = self.pointer_anchor else {
			return;
		};
		let Some(anchor_index) = layout
			.cluster_at_or_after(anchor_byte)
			.or_else(|| layout.cluster_before(anchor_byte.saturating_add(1)))
		else {
			return;
		};
		let Some(target_index) = self.pointer_cluster_index(layout, position) else {
			return;
		};

		self.select_range(layout, anchor_index, target_index);
	}

	fn select_range(&mut self, layout: &BufferLayoutSnapshot, anchor_index: usize, target_index: usize) {
		let Some(anchor) = layout.cluster(anchor_index) else {
			return;
		};
		let Some(target) = layout.cluster(target_index) else {
			return;
		};
		let start = anchor.byte_range.start.min(target.byte_range.start);
		let end = anchor.byte_range.end.max(target.byte_range.end);
		self.mode = EditorMode::Normal;
		self.selection = Some(start..end);
		self.caret = target.byte_range.start;
		self.preferred_x = Some(target.center_x());
	}

	fn select_word_at(&mut self, layout: &BufferLayoutSnapshot, position: Point) {
		let Some(cluster_index) = self.pointer_cluster_index(layout, position) else {
			return;
		};
		let Some(cluster) = layout.cluster(cluster_index) else {
			return;
		};
		let range = self.word_range(cluster.byte_range.clone());
		self.mode = EditorMode::Normal;
		self.selection = Some(range.clone());
		self.caret = range.start;
		self.preferred_x = Some(cluster.center_x());
		self.pointer_anchor = None;
	}

	fn word_range(&self, fallback: Range<usize>) -> Range<usize> {
		let Some(slice) = self.text.get(fallback.clone()) else {
			return fallback;
		};

		if !slice.chars().any(is_word_char) {
			return fallback;
		}

		let mut start = fallback.start;
		while let Some((index, ch)) = previous_char(&self.text, start) {
			if !is_word_char(ch) {
				break;
			}
			start = index;
		}

		let mut end = fallback.end;
		while let Some((next, ch)) = next_char(&self.text, end) {
			if !is_word_char(ch) {
				break;
			}
			end = next;
		}

		start..end
	}

	fn preview_range(&self, range: &Range<usize>) -> String {
		self.text
			.get(range.clone())
			.map(debug_snippet)
			.unwrap_or_else(|| "<invalid utf8 slice>".to_string())
	}

	fn apply_buffer_edit(&mut self, font_system: &mut FontSystem, edit: &TextEdit) {
		let start = byte_to_cursor(&self.text, edit.range.start);
		let end = byte_to_cursor(&self.text, edit.range.end);
		let buffer = Arc::make_mut(&mut self.buffer);
		let mut editor = CosmicEditor::new(&mut *buffer);

		editor.set_cursor(start);
		if start != end {
			editor.delete_range(start, end);
			editor.set_cursor(start);
		}
		if !edit.inserted.is_empty() {
			let _ = editor.insert_at(start, &edit.inserted, None);
		}

		buffer.shape_until_scroll(font_system, false);
	}

	fn width_only_config_change(&self, config: SceneConfig) -> bool {
		self.config.font_choice == config.font_choice
			&& self.config.shaping == config.shaping
			&& self.config.wrapping == config.wrapping
			&& self.config.render_mode == config.render_mode
			&& (self.config.font_size - config.font_size).abs() < f32::EPSILON
			&& (self.config.line_height - config.line_height).abs() < f32::EPSILON
	}

	fn resize_buffer(&mut self, font_system: &mut FontSystem, width: f32) {
		let buffer = Arc::make_mut(&mut self.buffer);
		buffer.set_size(font_system, Some(width), None);
		buffer.shape_until_scroll(font_system, false);
	}

	fn refresh_view_state(&mut self) {
		let layout = self.layout_snapshot();
		self.view_state = EditorViewState {
			mode: self.mode,
			selection: self.selection.clone(),
			caret: self.caret,
			selection_rectangles: self
				.selection
				.as_ref()
				.map(|selection| layout.selection_rectangles(selection))
				.unwrap_or_else(|| Arc::from([])),
			caret_geometry: layout.caret_geometry(self.caret, self.config.line_height),
		};
	}
}

#[derive(Debug, Clone)]
struct BufferRunInfo {
	cluster_range: Range<usize>,
	line_height: f32,
	line_top: f32,
}

#[derive(Debug, Clone)]
struct BufferClusterInfo {
	byte_range: Range<usize>,
	height: f32,
	run_index: usize,
	width: f32,
	x: f32,
	y: f32,
}

impl BufferClusterInfo {
	fn center_x(&self) -> f32 {
		self.x + (self.width * 0.5)
	}
}

impl EditorSelectionRect {
	fn from_span(start: &BufferClusterInfo, end: &BufferClusterInfo) -> Self {
		Self {
			x: start.x,
			y: start.y.min(end.y),
			width: (end.x + end.width) - start.x,
			height: start.height.max(end.height),
		}
	}
}

#[derive(Debug, Clone)]
struct BufferCaretMetrics {
	run_index: usize,
	x: f32,
	y: f32,
	height: f32,
}

#[derive(Debug, Clone)]
struct BufferLayoutSnapshot {
	clusters: Vec<BufferClusterInfo>,
	line_byte_offsets: Vec<usize>,
	runs: Vec<BufferRunInfo>,
}

impl BufferLayoutSnapshot {
	fn new(buffer: &Buffer, text: &str) -> Self {
		let line_byte_offsets = line_byte_offsets(text);
		let mut runs = Vec::new();
		let mut clusters = Vec::new();

		for run in buffer.layout_runs() {
			let line_byte_offset = line_byte_offsets[run.line_i];
			let cluster_start = clusters.len();
			clusters.extend(build_buffer_clusters(
				runs.len(),
				line_byte_offset,
				run.line_top,
				run.line_height,
				run.glyphs,
			));
			let cluster_end = clusters.len();

			runs.push(BufferRunInfo {
				cluster_range: cluster_start..cluster_end,
				line_height: run.line_height,
				line_top: run.line_top,
			});
		}

		Self {
			clusters,
			line_byte_offsets,
			runs,
		}
	}

	fn clusters(&self) -> &[BufferClusterInfo] {
		&self.clusters
	}

	fn cluster(&self, index: usize) -> Option<&BufferClusterInfo> {
		self.clusters.get(index)
	}

	fn cluster_index_for_cursor(&self, cursor: Cursor) -> Option<usize> {
		if self.clusters.is_empty() {
			return None;
		}

		let line_offset = self.line_byte_offsets.get(cursor.line).copied().unwrap_or_default();
		let byte = line_offset + cursor.index;

		if cursor.affinity.before() {
			self.clusters
				.iter()
				.enumerate()
				.find(|(_, cluster)| cluster.byte_range.end == byte)
				.map(|(index, _)| index)
				.or_else(|| self.cluster_before(byte.saturating_add(1)))
		} else {
			self.cluster_at_or_after(byte)
				.filter(|index| self.clusters[*index].byte_range.start <= byte)
				.or_else(|| self.cluster_before(byte))
		}
	}

	fn cluster_at_or_after(&self, byte: usize) -> Option<usize> {
		let index = self.clusters.partition_point(|cluster| cluster.byte_range.end <= byte);
		(index < self.clusters.len()).then_some(index)
	}

	fn cluster_before(&self, byte: usize) -> Option<usize> {
		self.clusters
			.partition_point(|cluster| cluster.byte_range.start < byte)
			.checked_sub(1)
	}

	fn first_cluster_in_run(&self, run_index: usize) -> Option<usize> {
		self.runs
			.get(run_index)
			.and_then(|run| (!run.cluster_range.is_empty()).then_some(run.cluster_range.start))
	}

	fn last_cluster_in_run(&self, run_index: usize) -> Option<usize> {
		self.runs
			.get(run_index)
			.and_then(|run| (!run.cluster_range.is_empty()).then_some(run.cluster_range.end - 1))
	}

	fn nearest_cluster_on_adjacent_run(&self, run_index: usize, preferred_x: f32, direction: isize) -> Option<usize> {
		let mut next = run_index as isize + direction;

		while next >= 0 && next < self.runs.len() as isize {
			if let Some(target) = self.nearest_cluster_in_run(next as usize, preferred_x) {
				return Some(target);
			}
			next += direction;
		}

		None
	}

	fn nearest_cluster_in_run(&self, run_index: usize, preferred_x: f32) -> Option<usize> {
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

	fn nearest_cluster_at(&self, y: f32, preferred_x: f32) -> Option<usize> {
		let run_index = self
			.runs
			.iter()
			.enumerate()
			.min_by(|(_, a), (_, b)| {
				run_distance(a, y)
					.total_cmp(&run_distance(b, y))
					.then_with(|| a.line_top.total_cmp(&b.line_top))
			})
			.map(|(index, _)| index)?;
		self.nearest_cluster_in_run(run_index, preferred_x)
	}

	fn selection_rectangles(&self, range: &Range<usize>) -> Arc<[EditorSelectionRect]> {
		let selected = self
			.clusters
			.iter()
			.filter(|cluster| cluster.byte_range.end > range.start && cluster.byte_range.start < range.end)
			.collect::<Vec<_>>();
		if selected.is_empty() {
			return Arc::from([]);
		}

		let mut rectangles = Vec::new();
		let mut span_start = selected[0];
		let mut span_end = selected[0];

		for cluster in selected.into_iter().skip(1) {
			let same_run = cluster.run_index == span_end.run_index;
			let contiguous = cluster.byte_range.start <= span_end.byte_range.end;
			if same_run && contiguous {
				span_end = cluster;
				continue;
			}

			rectangles.push(EditorSelectionRect::from_span(span_start, span_end));
			span_start = cluster;
			span_end = cluster;
		}

		rectangles.push(EditorSelectionRect::from_span(span_start, span_end));
		rectangles.into()
	}

	fn caret_metrics(&self, byte: usize, fallback_height: f32) -> BufferCaretMetrics {
		if self.clusters.is_empty() {
			return BufferCaretMetrics {
				run_index: 0,
				x: 0.0,
				y: 0.0,
				height: fallback_height.max(1.0),
			};
		}

		if let Some(index) = self.cluster_at_or_after(byte) {
			let cluster = &self.clusters[index];
			if byte <= cluster.byte_range.start {
				return BufferCaretMetrics {
					run_index: cluster.run_index,
					x: cluster.x,
					y: cluster.y,
					height: cluster.height.max(1.0),
				};
			}
		}

		if let Some(index) = self.cluster_before(byte) {
			let cluster = &self.clusters[index];
			return BufferCaretMetrics {
				run_index: cluster.run_index,
				x: cluster.x + cluster.width,
				y: cluster.y,
				height: cluster.height.max(1.0),
			};
		}

		let run = &self.runs[0];
		BufferCaretMetrics {
			run_index: 0,
			x: 0.0,
			y: run.line_top,
			height: run.line_height.max(1.0),
		}
	}

	fn caret_geometry(&self, byte: usize, fallback_height: f32) -> EditorCaretGeometry {
		let metrics = self.caret_metrics(byte, fallback_height);
		EditorCaretGeometry {
			x: metrics.x,
			y: metrics.y,
			height: metrics.height,
		}
	}
}

fn build_buffer_clusters(
	run_index: usize, line_byte_offset: usize, line_top: f32, line_height: f32, glyphs: &[cosmic_text::LayoutGlyph],
) -> Vec<BufferClusterInfo> {
	let mut clusters = Vec::new();
	let mut current: Option<BufferClusterInfo> = None;

	for glyph in glyphs {
		let byte_range = (line_byte_offset + glyph.start)..(line_byte_offset + glyph.end);
		let glyph_y = line_top + glyph.y;
		let glyph_height = glyph.line_height_opt.unwrap_or(line_height);

		match current.as_mut() {
			Some(cluster) if cluster.byte_range == byte_range => {
				cluster.width = (glyph.x + glyph.w - cluster.x).max(cluster.width);
				cluster.height = cluster.height.max(glyph_height);
				cluster.y = cluster.y.min(glyph_y);
			}
			_ => {
				if let Some(cluster) = current.take() {
					clusters.push(cluster);
				}

				current = Some(BufferClusterInfo {
					byte_range,
					height: glyph_height.max(1.0),
					run_index,
					width: glyph.w.max(1.0),
					x: glyph.x,
					y: glyph_y,
				});
			}
		}
	}

	if let Some(cluster) = current {
		clusters.push(cluster);
	}

	clusters
}

fn clamp_char_boundary(text: &str, byte: usize) -> usize {
	if byte >= text.len() {
		return text.len();
	}

	let mut boundary = byte;
	while boundary > 0 && !text.is_char_boundary(boundary) {
		boundary -= 1;
	}
	boundary
}

fn previous_char_boundary(text: &str, byte: usize) -> Option<usize> {
	let byte = clamp_char_boundary(text, byte);
	text[..byte].char_indices().last().map(|(index, _)| index)
}

fn next_char_boundary(text: &str, byte: usize) -> Option<usize> {
	let byte = clamp_char_boundary(text, byte);
	text[byte..]
		.char_indices()
		.nth(1)
		.map(|(offset, _)| byte + offset)
		.or_else(|| (byte < text.len()).then_some(text.len()))
}

fn previous_char(text: &str, byte: usize) -> Option<(usize, char)> {
	let byte = clamp_char_boundary(text, byte);
	text[..byte].char_indices().last()
}

fn next_char(text: &str, byte: usize) -> Option<(usize, char)> {
	let byte = clamp_char_boundary(text, byte);
	let (_, ch) = text[byte..].char_indices().next()?;
	Some((byte + ch.len_utf8(), ch))
}

fn is_word_char(ch: char) -> bool {
	ch.is_alphanumeric() || ch == '_'
}

fn byte_to_cursor(text: &str, byte: usize) -> Cursor {
	let mut clamped = byte.min(text.len());
	while clamped > 0 && !text.is_char_boundary(clamped) {
		clamped -= 1;
	}

	let line_offsets = line_byte_offsets(text);
	let line = line_offsets
		.partition_point(|offset| *offset <= clamped)
		.saturating_sub(1);
	Cursor::new(line, clamped - line_offsets[line])
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

fn run_distance(run: &BufferRunInfo, y: f32) -> f32 {
	let run_bottom = run.line_top + run.line_height;
	if y < run.line_top {
		run.line_top - y
	} else if y > run_bottom {
		y - run_bottom
	} else {
		0.0
	}
}

fn debug_snippet(text: &str) -> String {
	let escaped: String = text.chars().flat_map(char::escape_default).collect();
	if escaped.is_empty() {
		"<empty>".to_string()
	} else {
		format!("\"{escaped}\"")
	}
}

#[cfg(test)]
mod tests {
	use super::{EditorBuffer, EditorCommand, EditorMode};
	use crate::scene::{LayoutScene, make_font_system, scene_config};
	use crate::types::{FontChoice, RenderMode, ShapingChoice, WrapChoice};
	use iced::Point;

	fn editor(text: &str) -> (cosmic_text::FontSystem, EditorBuffer) {
		let mut font_system = make_font_system();
		let config = scene_config(
			FontChoice::SansSerif,
			ShapingChoice::Advanced,
			WrapChoice::Word,
			RenderMode::CanvasOnly,
			24.0,
			32.0,
			400.0,
		);
		let editor = EditorBuffer::new(&mut font_system, text, config);
		(font_system, editor)
	}

	#[test]
	fn normal_mode_moves_by_visual_cluster() {
		let (mut font_system, mut editor) = editor("ab\nd");

		assert_eq!(editor.view_state().selection, Some(0..1));

		editor.apply(&mut font_system, EditorCommand::MoveRight);
		assert_eq!(editor.view_state().selection, Some(1..2));

		editor.apply(&mut font_system, EditorCommand::MoveDown);
		assert_eq!(editor.view_state().selection, Some(3..4));
	}

	#[test]
	fn insert_mode_backspace_keeps_caret_on_char_boundaries() {
		let (mut font_system, mut editor) = editor("aé");

		editor.apply(&mut font_system, EditorCommand::EnterInsertAfter);
		assert_eq!(editor.view_state().mode, EditorMode::Insert);

		editor.apply(&mut font_system, EditorCommand::Backspace);
		assert_eq!(editor.text(), "é");
		assert_eq!(editor.buffer_text(), "é");
		assert_eq!(editor.view_state().caret, 0);
	}

	#[test]
	fn escape_from_insert_returns_to_normal_selection() {
		let (mut font_system, mut editor) = editor("abc");

		editor.apply(&mut font_system, EditorCommand::EnterInsertAfter);
		editor.apply(&mut font_system, EditorCommand::MoveRight);
		editor.apply(&mut font_system, EditorCommand::ExitInsert);

		assert_eq!(editor.view_state().mode, EditorMode::Normal);
		assert_eq!(editor.view_state().selection, Some(1..2));
	}

	#[test]
	fn delete_selection_on_later_line_handles_multibyte_text() {
		let text = "🙂\né";
		let mut font_system = make_font_system();
		let scene = LayoutScene::build(
			&mut font_system,
			text.to_string(),
			FontChoice::SansSerif,
			ShapingChoice::Advanced,
			WrapChoice::None,
			24.0,
			32.0,
			400.0,
			RenderMode::CanvasAndOutlines,
		);
		let config = scene_config(
			FontChoice::SansSerif,
			ShapingChoice::Advanced,
			WrapChoice::None,
			RenderMode::CanvasAndOutlines,
			24.0,
			32.0,
			400.0,
		);
		let mut editor = EditorBuffer::new(&mut font_system, text, config);

		assert_eq!(
			editor
				.view_state()
				.selection
				.as_ref()
				.and_then(|selection| scene.text.get(selection.clone())),
			Some("🙂")
		);

		editor.apply(&mut font_system, EditorCommand::MoveDown);
		assert_eq!(
			editor
				.view_state()
				.selection
				.as_ref()
				.and_then(|selection| scene.text.get(selection.clone())),
			Some("é")
		);

		assert!(editor.apply(&mut font_system, EditorCommand::DeleteSelection).changed);
		assert_eq!(editor.text(), "🙂\n");
		assert_eq!(editor.buffer_text(), "🙂\n");
	}

	#[test]
	fn live_selection_rectangles_track_wrapped_width_changes() {
		let text = "alpha beta gamma delta epsilon zeta eta theta";
		let (mut font_system, mut editor) = editor(text);

		for _ in 0..14 {
			editor.apply(&mut font_system, EditorCommand::MoveRight);
		}

		let before = editor
			.view_state()
			.selection_rectangles
			.first()
			.copied()
			.expect("selection geometry should exist before resize");

		editor.sync_buffer_width(&mut font_system, 110.0);

		let after = editor
			.view_state()
			.selection_rectangles
			.first()
			.copied()
			.expect("selection geometry should exist after resize");

		assert!(
			after.y > before.y || (after.y == before.y && after.x < before.x),
			"expected wrapped selection to move after width shrink, before={before:?} after={after:?}"
		);
	}

	#[test]
	fn drag_selection_spans_multiple_wrapped_rectangles() {
		let text = "alpha beta gamma delta epsilon zeta eta theta";
		let (mut font_system, mut editor) = editor(text);
		editor.sync_buffer_width(&mut font_system, 130.0);

		let start = editor
			.view_state()
			.selection_rectangles
			.first()
			.copied()
			.expect("initial selection should have a rectangle");

		editor.apply(
			&mut font_system,
			EditorCommand::BeginPointerSelection {
				position: Point::new(start.x + 2.0, start.y + 2.0),
				select_word: false,
			},
		);
		editor.apply(
			&mut font_system,
			EditorCommand::DragPointerSelection(Point::new(90.0, 120.0)),
		);
		editor.apply(&mut font_system, EditorCommand::EndPointerSelection);

		let view = editor.view_state();
		assert!(view.selection_rectangles.len() >= 2);
		assert!(
			view.selection
				.as_ref()
				.is_some_and(|selection| selection.end > selection.start)
		);
	}

	#[test]
	fn double_click_selects_a_full_word() {
		let (mut font_system, mut editor) = editor("alpha beta gamma");

		for _ in 0..11 {
			editor.apply(&mut font_system, EditorCommand::MoveRight);
		}

		let rect = editor
			.view_state()
			.selection_rectangles
			.first()
			.copied()
			.expect("selection should have a rectangle");

		editor.apply(
			&mut font_system,
			EditorCommand::BeginPointerSelection {
				position: Point::new(rect.x + 2.0, rect.y + 2.0),
				select_word: true,
			},
		);

		let selection = editor
			.view_state()
			.selection
			.expect("double click should produce a selection");
		assert_eq!(editor.text().get(selection), Some("gamma"));
	}
}
