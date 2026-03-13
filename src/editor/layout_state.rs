use cosmic_text::{Buffer, Cursor, Edit as _, Editor as CosmicEditor, FontSystem};
use iced::Point;

use std::sync::Arc;

use crate::scene::{SceneConfig, build_buffer};

use super::layout::BufferLayoutSnapshot;
use super::text::byte_to_cursor;
use super::{EditorMode, EditorSelectionRect, EditorViewState, TextEdit};

#[derive(Debug, Clone)]
pub(super) struct EditorLayout {
	buffer: Arc<Buffer>,
	config: SceneConfig,
	view_state: EditorViewState,
}

#[derive(Debug, Clone, Copy)]
struct InsertCursorGeometry {
	x: f32,
	y: f32,
	height: f32,
	block_width: f32,
}

impl EditorLayout {
	pub(super) fn new(font_system: &mut FontSystem, text: &str, config: SceneConfig) -> Self {
		Self {
			buffer: Arc::new(build_buffer(font_system, text, config)),
			config,
			view_state: EditorViewState {
				mode: EditorMode::Normal,
				selection: None,
				selection_head: None,
				selection_rectangles: Arc::from([]),
				caret_rectangle: None,
				viewport_target: None,
			},
		}
	}

	pub(super) fn buffer(&self) -> Arc<Buffer> {
		self.buffer.clone()
	}

	pub(super) fn view_state(&self) -> EditorViewState {
		self.view_state.clone()
	}

	pub(super) fn view_state_ref(&self) -> &EditorViewState {
		&self.view_state
	}

	pub(super) fn set_view_state(&mut self, view_state: EditorViewState) {
		self.view_state = view_state;
	}

	pub(super) fn sync_buffer_config(&mut self, font_system: &mut FontSystem, text: &str, config: SceneConfig) {
		if self.config == config {
			return;
		}

		if self.width_only_config_change(config) {
			self.resize_buffer(font_system, config.max_width);
			self.config = config;
			return;
		}

		self.config = config;
		self.buffer = Arc::new(build_buffer(font_system, text, config));
	}

	pub(super) fn sync_buffer_width(&mut self, font_system: &mut FontSystem, width: f32) {
		if (self.config.max_width - width).abs() < 0.5 {
			return;
		}

		self.resize_buffer(font_system, width);
		self.config.max_width = width;
	}

	pub(super) fn reset(&mut self, font_system: &mut FontSystem, text: &str, config: SceneConfig) {
		self.config = config;
		self.buffer = Arc::new(build_buffer(font_system, text, config));
		self.view_state = EditorViewState {
			mode: EditorMode::Normal,
			selection: None,
			selection_head: None,
			selection_rectangles: Arc::from([]),
			caret_rectangle: None,
			viewport_target: None,
		};
	}

	pub(super) fn snapshot(&self, text: &str) -> BufferLayoutSnapshot {
		BufferLayoutSnapshot::new(&self.buffer, text)
	}

	pub(super) fn hit(&self, point: Point) -> Option<Cursor> {
		self.buffer.hit(point.x, point.y)
	}

	pub(super) fn insert_cursor_rectangle(&self, text: &str, byte: usize) -> Option<EditorSelectionRect> {
		let geometry = self.insert_cursor_geometry(text, byte)?;

		Some(EditorSelectionRect {
			x: geometry.x,
			y: geometry.y,
			width: 2.0,
			height: geometry.height,
		})
	}

	pub(super) fn insert_cursor_block(&self, text: &str, byte: usize) -> Option<EditorSelectionRect> {
		let geometry = self.insert_cursor_geometry(text, byte)?;

		Some(EditorSelectionRect {
			x: geometry.x,
			y: geometry.y,
			width: geometry.block_width.max(2.0),
			height: geometry.height,
		})
	}

	pub(super) fn apply_edit(&mut self, font_system: &mut FontSystem, text: &str, edit: &TextEdit) {
		if edit_changes_line_structure(text, edit) {
			let mut next_text = text.to_string();
			next_text.replace_range(edit.range.clone(), &edit.inserted);
			self.buffer = Arc::new(build_buffer(font_system, &next_text, self.config));
			return;
		}

		let start = byte_to_cursor(text, edit.range.start);
		let end = byte_to_cursor(text, edit.range.end);
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

	#[cfg(test)]
	pub(super) fn buffer_text(&self) -> String {
		let mut text = String::new();
		for line in &self.buffer.lines {
			text.push_str(line.text());
			text.push_str(line.ending().as_str());
		}
		text
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

	fn insert_cursor_geometry(&self, text: &str, byte: usize) -> Option<InsertCursorGeometry> {
		let cursor = byte_to_cursor(text, byte);
		let line_height = self.buffer.metrics().line_height.max(1.0);
		let default_width = (self.config.font_size * 0.6).max(2.0);
		let scroll = self.buffer.scroll();
		let line = self.buffer.lines.get(cursor.line)?;
		let layout = line.layout_opt()?;

		let (visual_line, offset, block_width) = layout
			.iter()
			.enumerate()
			.find_map(|(index, line)| {
				let start = line.glyphs.first().map(|glyph| glyph.start).unwrap_or(0);
				let end = line.glyphs.last().map(|glyph| glyph.end).unwrap_or(0);
				let is_cursor_before_start = start > cursor.index;
				let is_cursor_before_end = cursor.index <= end;

				if is_cursor_before_start {
					index.checked_sub(1).map(|previous| {
						let previous_line = &layout[previous];
						let width = previous_line
							.glyphs
							.last()
							.map(|glyph| glyph.w.max(2.0))
							.unwrap_or(default_width);

						(previous, previous_line.w, width)
					})
				} else if is_cursor_before_end {
					let offset = line
						.glyphs
						.iter()
						.take_while(|glyph| cursor.index > glyph.start)
						.map(|glyph| glyph.w)
						.sum();
					let width = line
						.glyphs
						.iter()
						.find(|glyph| cursor.index <= glyph.start)
						.or_else(|| line.glyphs.last())
						.map(|glyph| glyph.w.max(2.0))
						.unwrap_or(default_width);

					Some((index, offset, width))
				} else {
					None
				}
			})
			.unwrap_or((
				layout.len().saturating_sub(1),
				layout.last().map(|line| line.w).unwrap_or(0.0),
				layout
					.last()
					.and_then(|line| line.glyphs.last())
					.map(|glyph| glyph.w.max(2.0))
					.unwrap_or(default_width),
			));
		let y = (visual_lines_offset(cursor.line, &self.buffer) + visual_line as i32) as f32 * line_height
			- scroll.vertical;

		Some(InsertCursorGeometry {
			x: offset,
			y,
			height: line_height,
			block_width,
		})
	}
}

fn edit_changes_line_structure(text: &str, edit: &TextEdit) -> bool {
	// This editor's hard-line model treats only `\n` as a structural line break.
	text.get(edit.range.clone())
		.is_some_and(|removed| removed.contains('\n'))
		|| edit.inserted.contains('\n')
}

fn visual_lines_offset(line: usize, buffer: &Buffer) -> i32 {
	let scroll = buffer.scroll();
	let start = scroll.line.min(line);
	let end = scroll.line.max(line);
	let visual_lines: usize = buffer.lines[start..]
		.iter()
		.take(end - start)
		.map(|line| line.layout_opt().map(Vec::len).unwrap_or_default())
		.sum();

	visual_lines as i32 * if scroll.line < line { 1 } else { -1 }
}
