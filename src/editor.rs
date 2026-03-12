use cosmic_text::{Buffer, Cursor, Edit as _, Editor as CosmicEditor, FontSystem};
use iced::Point;

use std::fmt::{self, Display};
use std::ops::Range;
use std::sync::Arc;

use crate::scene::{LayoutScene, SceneConfig, build_buffer};

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
}

#[derive(Debug, Clone)]
pub(crate) enum EditorCommand {
	SelectClusterAt(Point),
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
		};
		editor.reset_normal_selection();
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
			return;
		}

		self.config = config;
		self.buffer = Arc::new(build_buffer(font_system, &self.text, config));
	}

	pub(crate) fn sync_buffer_width(&mut self, font_system: &mut FontSystem, width: f32) {
		if (self.config.max_width - width).abs() < 0.5 {
			return;
		}

		self.resize_buffer(font_system, width);
		self.config.max_width = width;
	}

	pub(crate) fn reset(&mut self, font_system: &mut FontSystem, text: impl Into<String>, config: SceneConfig) {
		self.text = text.into();
		self.config = config;
		self.buffer = Arc::new(build_buffer(font_system, &self.text, config));
		self.mode = EditorMode::Normal;
		self.selection = None;
		self.caret = 0;
		self.preferred_x = None;
		self.reset_normal_selection();
	}

	pub(crate) fn view_state(&self) -> EditorViewState {
		EditorViewState {
			mode: self.mode,
			selection: self.selection.clone(),
			caret: self.caret,
		}
	}

	pub(crate) fn apply(&mut self, font_system: &mut FontSystem, command: EditorCommand) -> ApplyResult {
		match command {
			EditorCommand::SelectClusterAt(point) => {
				let layout = self.layout_snapshot();
				if let Some(cluster_index) = self
					.buffer
					.hit(point.x, point.y)
					.and_then(|cursor| layout.cluster_index_for_cursor(cursor))
				{
					self.select_cluster(&layout, cluster_index);
				} else if self.text.is_empty() {
					self.mode = EditorMode::Insert;
					self.caret = 0;
					self.preferred_x = None;
				}
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
				let layout = self.layout_snapshot();
				self.mode = EditorMode::Insert;
				self.caret = self
					.current_selection(&layout)
					.map(|cluster| cluster.byte_range.start)
					.unwrap_or(0);
				self.preferred_x = None;
				ApplyResult {
					changed: false,
					text_edit: None,
				}
			}
			EditorCommand::EnterInsertAfter => {
				let layout = self.layout_snapshot();
				self.mode = EditorMode::Insert;
				self.caret = self
					.current_selection(&layout)
					.map(|cluster| cluster.byte_range.end)
					.unwrap_or(self.text.len());
				self.preferred_x = None;
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
		}
	}

	pub(crate) fn selection_details(&self, scene: &LayoutScene) -> String {
		match self.mode {
			EditorMode::Normal => {
				let Some(cluster) = self
					.selection
					.as_ref()
					.and_then(|selection| scene.cluster_index_for_range(selection))
					.and_then(|index| scene.cluster(index))
				else {
					return format!("  mode: {}\n  selection: none", self.mode);
				};

				format!(
					"  mode: {}\n  cluster: {}\n  bytes: {:?}\n  run: {}\n  x/y: {:.1}, {:.1}\n  w/h: {:.1}, {:.1}",
					self.mode,
					scene.cluster_preview(cluster),
					cluster.byte_range,
					cluster.run_index,
					cluster.x,
					cluster.y,
					cluster.width,
					cluster.height,
				)
			}
			EditorMode::Insert => format!(
				"  mode: {}\n  caret byte: {}\n  selection on escape: {}",
				self.mode,
				self.caret,
				scene
					.cluster_before(self.caret)
					.or_else(|| scene.cluster_at_or_after(self.caret))
					.and_then(|index| scene.cluster(index))
					.map(|cluster| scene.cluster_preview(cluster))
					.unwrap_or_else(|| "<none>".to_string())
			),
		}
	}

	fn move_left(&mut self, layout: &BufferLayoutSnapshot) {
		match self.mode {
			EditorMode::Normal => {
				let Some(current) = self.selection_index(layout) else {
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
		match self.mode {
			EditorMode::Normal => {
				let Some(current) = self.selection_index(layout) else {
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
		match self.mode {
			EditorMode::Normal => {
				let Some(current) = self.current_selection(layout) else {
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
				let caret = layout.caret_metrics(self.caret);
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
		match self.mode {
			EditorMode::Normal => {
				let Some(current) = self.current_selection(layout) else {
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
				let caret = layout.caret_metrics(self.caret);
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

		self.selection = layout
			.cluster_before(self.caret)
			.or_else(|| layout.cluster_at_or_after(self.caret))
			.and_then(|index| layout.cluster(index))
			.map(|cluster| cluster.byte_range.clone());
	}

	fn delete_selection(&mut self, font_system: &mut FontSystem) -> ApplyResult {
		let layout = self.layout_snapshot();
		let Some(selection) = self
			.current_selection(&layout)
			.map(|cluster| cluster.byte_range.clone())
		else {
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
	}

	fn selection_index(&self, layout: &BufferLayoutSnapshot) -> Option<usize> {
		self.selection
			.as_ref()
			.and_then(|selection| layout.cluster_index_for_range(selection))
	}

	fn current_selection<'a>(&self, layout: &'a BufferLayoutSnapshot) -> Option<&'a BufferClusterInfo> {
		self.selection_index(layout).and_then(|index| layout.cluster(index))
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
}

#[derive(Debug, Clone)]
struct BufferRunInfo {
	cluster_range: Range<usize>,
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

#[derive(Debug, Clone)]
struct BufferCaretMetrics {
	run_index: usize,
	x: f32,
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

	fn cluster_index_for_range(&self, range: &Range<usize>) -> Option<usize> {
		let index = self
			.clusters
			.binary_search_by_key(&range.start, |cluster| cluster.byte_range.start)
			.ok()?;
		(self.clusters[index].byte_range == *range).then_some(index)
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

	fn caret_metrics(&self, byte: usize) -> BufferCaretMetrics {
		if self.clusters.is_empty() {
			return BufferCaretMetrics { run_index: 0, x: 0.0 };
		}

		if let Some(index) = self.cluster_at_or_after(byte) {
			let cluster = &self.clusters[index];
			if byte <= cluster.byte_range.start {
				return BufferCaretMetrics {
					run_index: cluster.run_index,
					x: cluster.x,
				};
			}
		}

		if let Some(index) = self.cluster_before(byte) {
			let cluster = &self.clusters[index];
			return BufferCaretMetrics {
				run_index: cluster.run_index,
				x: cluster.x + cluster.width,
			};
		}

		let cluster = &self.clusters[0];
		BufferCaretMetrics {
			run_index: cluster.run_index,
			x: cluster.x,
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

#[cfg(test)]
mod tests {
	use super::{EditorBuffer, EditorCommand, EditorMode};
	use crate::scene::{LayoutScene, make_font_system, scene_config};
	use crate::types::{FontChoice, RenderMode, ShapingChoice, WrapChoice};

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
}
