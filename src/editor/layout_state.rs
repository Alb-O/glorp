use {
	super::{
		EditorMode, EditorTextLayerState, EditorViewState, EditorViewportMetrics, TextEdit,
		geometry::{insert_cursor_block, insert_cursor_rectangle},
		layout::BufferLayoutSnapshot,
		text::{byte_to_cursor, byte_to_line_column},
	},
	crate::{
		overlay::LayoutRect,
		scene::{SceneConfig, build_buffer},
		telemetry::duration_ms,
	},
	cosmic_text::{Buffer, Cursor, Edit as _, Editor as CosmicEditor, FontSystem},
	iced::{
		Point,
		advanced::graphics::{text as iced_text, text::cosmic_text::Edit as _},
	},
	std::{sync::Arc, time::Instant},
	tracing::{debug, trace},
};

#[derive(Debug, Clone)]
pub(super) struct EditorLayout {
	buffer: Arc<Buffer>,
	render_buffer: Arc<iced_text::cosmic_text::Buffer>,
	config: SceneConfig,
	view_state: EditorViewState,
}

impl EditorLayout {
	pub(super) fn new(font_system: &mut FontSystem, text: &str, config: SceneConfig) -> Self {
		Self {
			buffer: Arc::new(build_buffer(font_system, text, config)),
			render_buffer: Arc::new(build_render_buffer(text, config)),
			config,
			view_state: default_view_state(),
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

	pub(super) fn sync_buffer_config(&mut self, font_system: &mut FontSystem, text: &str, config: SceneConfig) -> bool {
		if self.config == config {
			return false;
		}

		if self.width_only_config_change(config) {
			self.resize_buffer(font_system, config.max_width);
			self.resize_render_buffer(config.max_width);
			self.config = config;
			return true;
		}

		self.config = config;
		self.buffer = Arc::new(build_buffer(font_system, text, config));
		self.render_buffer = Arc::new(build_render_buffer(text, config));
		true
	}

	pub(super) fn sync_buffer_width(&mut self, font_system: &mut FontSystem, width: f32) {
		if (self.config.max_width - width).abs() < 0.5 {
			return;
		}

		self.resize_buffer(font_system, width);
		self.resize_render_buffer(width);
		self.config.max_width = width;
	}

	pub(super) fn reset(&mut self, font_system: &mut FontSystem, text: &str, config: SceneConfig) {
		self.config = config;
		self.buffer = Arc::new(build_buffer(font_system, text, config));
		self.render_buffer = Arc::new(build_render_buffer(text, config));
		self.view_state = default_view_state();
	}

	pub(super) fn snapshot(&self, text: &str) -> BufferLayoutSnapshot {
		BufferLayoutSnapshot::new(&self.buffer, text)
	}

	pub(super) fn viewport_metrics(&self) -> EditorViewportMetrics {
		let (measured_width, measured_height) = measure_buffer(&self.buffer);
		EditorViewportMetrics {
			wrapping: self.config.wrapping,
			measured_width,
			measured_height,
		}
	}

	pub(super) fn text_layer_state(&self) -> EditorTextLayerState {
		let metrics = self.viewport_metrics();
		EditorTextLayerState {
			buffer: Arc::downgrade(&self.render_buffer),
			render_mode: self.config.render_mode,
			measured_height: metrics.measured_height,
		}
	}

	pub(super) fn hit(&self, point: Point) -> Option<Cursor> {
		self.buffer.hit(point.x, point.y)
	}

	pub(super) fn insert_cursor_rectangle(&self, text: &str, byte: usize) -> Option<LayoutRect> {
		insert_cursor_rectangle(&self.buffer, self.config.font_size, text, byte)
	}

	pub(super) fn insert_cursor_block(&self, text: &str, byte: usize) -> Option<LayoutRect> {
		insert_cursor_block(&self.buffer, self.config.font_size, text, byte)
	}

	pub(super) fn apply_edit(&mut self, font_system: &mut FontSystem, text: &str, edit: &TextEdit) {
		let started = Instant::now();
		if edit_changes_line_structure(text, edit) {
			let mut next_text = text.to_string();
			next_text.replace_range(edit.range.clone(), &edit.inserted);
			self.buffer = Arc::new(build_buffer(font_system, &next_text, self.config));
			self.render_buffer = Arc::new(build_render_buffer(&next_text, self.config));
			debug!(
				duration_ms = duration_ms(started.elapsed()),
				text_bytes = next_text.len(),
				inserted_bytes = edit.inserted.len(),
				range_start = edit.range.start,
				range_end = edit.range.end,
				"layout edit rebuilt full buffer"
			);
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
		self.apply_render_buffer_edit(text, edit);
		trace!(
			duration_ms = duration_ms(started.elapsed()),
			text_bytes = text.len(),
			inserted_bytes = edit.inserted.len(),
			range_start = edit.range.start,
			range_end = edit.range.end,
			"layout edit updated retained buffer"
		);
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

	fn resize_render_buffer(&mut self, width: f32) {
		let buffer = Arc::make_mut(&mut self.render_buffer);
		let mut font_system = iced_text::font_system().write().expect("Write font system");
		buffer.set_size(font_system.raw(), Some(width), None);
		buffer.shape_until_scroll(font_system.raw(), false);
	}

	fn apply_render_buffer_edit(&mut self, text: &str, edit: &TextEdit) {
		let (start_line, start_index) = byte_to_line_column(text, edit.range.start);
		let (end_line, end_index) = byte_to_line_column(text, edit.range.end);
		let start = iced_text::cosmic_text::Cursor::new(start_line, start_index);
		let end = iced_text::cosmic_text::Cursor::new(end_line, end_index);
		let buffer = Arc::make_mut(&mut self.render_buffer);
		let mut editor = iced_text::cosmic_text::Editor::new(&mut *buffer);

		editor.set_cursor(start);
		if start != end {
			editor.delete_range(start, end);
			editor.set_cursor(start);
		}
		if !edit.inserted.is_empty() {
			let _ = editor.insert_at(start, &edit.inserted, None);
		}

		let mut font_system = iced_text::font_system().write().expect("Write font system");
		buffer.shape_until_scroll(font_system.raw(), false);
	}
}

fn edit_changes_line_structure(text: &str, edit: &TextEdit) -> bool {
	// This editor's hard-line model treats only `\n` as a structural line break.
	text.get(edit.range.clone())
		.is_some_and(|removed| removed.contains('\n'))
		|| edit.inserted.contains('\n')
}

fn default_view_state() -> EditorViewState {
	EditorViewState {
		mode: EditorMode::Normal,
		selection: None,
		selection_head: None,
		pointer_anchor: None,
		overlays: Arc::from([]),
		viewport_target: None,
	}
}

fn measure_buffer(buffer: &Buffer) -> (f32, f32) {
	let mut measured_width: f32 = 0.0;
	let mut measured_height: f32 = 0.0;

	for run in buffer.layout_runs() {
		measured_width = measured_width.max(run.line_w);
		measured_height = measured_height.max(run.line_top + run.line_height);
	}

	(measured_width, measured_height)
}

fn build_render_buffer(text: &str, config: SceneConfig) -> iced_text::cosmic_text::Buffer {
	let mut font_system = iced_text::font_system().write().expect("Write font system");
	let mut buffer = iced_text::cosmic_text::Buffer::new(
		font_system.raw(),
		iced_text::cosmic_text::Metrics::new(config.font_size, config.line_height),
	);

	buffer.set_size(font_system.raw(), Some(config.max_width), None);
	buffer.set_wrap(font_system.raw(), iced_text::to_wrap(config.wrapping.to_iced()));
	buffer.set_text(
		font_system.raw(),
		text,
		&iced_text::to_attributes(config.font()),
		iced_text::to_shaping(config.shaping.to_iced(), text),
		None,
	);
	buffer
}
