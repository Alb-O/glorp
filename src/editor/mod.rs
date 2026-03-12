mod editing;
mod layout;
mod navigation;
mod selection;
mod text;

#[cfg(test)]
mod tests;

use cosmic_text::{Buffer, FontSystem};
use iced::Point;

use std::fmt::{self, Display};
use std::ops::Range;
use std::sync::Arc;

use crate::scene::{SceneConfig, build_buffer};

use self::layout::{BufferClusterInfo, BufferLayoutSnapshot};
use self::text::debug_snippet;

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
	#[cfg(test)]
	pub(crate) selection: Option<Range<usize>>,
	#[cfg(test)]
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
				#[cfg(test)]
				selection: None,
				#[cfg(test)]
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

	#[cfg(test)]
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

	fn preview_range(&self, range: &Range<usize>) -> String {
		self.text
			.get(range.clone())
			.map(debug_snippet)
			.unwrap_or_else(|| "<invalid utf8 slice>".to_string())
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
			#[cfg(test)]
			selection: self.selection.clone(),
			#[cfg(test)]
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
