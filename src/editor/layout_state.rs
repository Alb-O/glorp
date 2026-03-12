use cosmic_text::{Buffer, Cursor, Edit as _, Editor as CosmicEditor, FontSystem};
use iced::Point;

use std::sync::Arc;

use crate::scene::{SceneConfig, build_buffer};

use super::layout::BufferLayoutSnapshot;
use super::text::byte_to_cursor;
use super::{EditorCaretGeometry, EditorMode, EditorViewState, TextEdit};

#[derive(Debug, Clone)]
pub(super) struct EditorLayout {
	buffer: Arc<Buffer>,
	config: SceneConfig,
	view_state: EditorViewState,
}

impl EditorLayout {
	pub(super) fn new(font_system: &mut FontSystem, text: &str, config: SceneConfig) -> Self {
		Self {
			buffer: Arc::new(build_buffer(font_system, text, config)),
			config,
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

	pub(super) fn line_height(&self) -> f32 {
		self.config.line_height
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
			viewport_target: None,
		};
	}

	pub(super) fn snapshot(&self, text: &str) -> BufferLayoutSnapshot {
		BufferLayoutSnapshot::new(&self.buffer, text)
	}

	pub(super) fn hit(&self, point: Point) -> Option<Cursor> {
		self.buffer.hit(point.x, point.y)
	}

	pub(super) fn apply_edit(&mut self, font_system: &mut FontSystem, text: &str, edit: &TextEdit) {
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
}
