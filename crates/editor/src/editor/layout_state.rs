use {
	super::{
		EditorTextLayerState, EditorViewState, EditorViewportMetrics, TextEdit,
		geometry::{insert_cursor_block, insert_cursor_rectangle},
		text::byte_to_cursor,
	},
	crate::{
		overlay::LayoutRect,
		scene::{DocumentLayout, FontNameMap, SceneConfig, build_buffer, resolve_font_names_from_buffer},
		telemetry::duration_ms,
	},
	cosmic_text::{Buffer, Cursor, Edit as _, Editor as CosmicEditor, FontSystem},
	iced::Point,
	std::{
		cell::{Cell, RefCell},
		sync::Arc,
		time::Instant,
	},
	tracing::{debug, trace},
};

#[derive(Debug, Clone)]
pub struct EditorLayout {
	buffer: Arc<Buffer>,
	config: SceneConfig,
	document_layout: RefCell<Option<Arc<DocumentLayout>>>,
	// Keep the resolved names next to the retained buffer so shared layout
	// rebuilds do not need a wider `FontSystem` dependency.
	font_names: RefCell<FontNameMap>,
	viewport_metrics: Cell<Option<EditorViewportMetrics>>,
	view_state: EditorViewState,
}

impl EditorLayout {
	pub fn new(font_system: &mut FontSystem, text: &str, config: SceneConfig) -> Self {
		let buffer = Arc::new(build_buffer(font_system, text, config));
		let font_names = resolve_font_names_from_buffer(font_system, &buffer);
		Self {
			font_names: RefCell::new(font_names),
			buffer,
			config,
			document_layout: RefCell::new(None),
			viewport_metrics: Cell::new(None),
			view_state: EditorViewState::default(),
		}
	}

	pub fn buffer(&self) -> Arc<Buffer> {
		Arc::clone(&self.buffer)
	}

	pub fn view_state(&self) -> EditorViewState {
		self.view_state.clone()
	}

	pub fn view_state_ref(&self) -> &EditorViewState {
		&self.view_state
	}

	pub fn set_view_state(&mut self, view_state: EditorViewState) {
		self.view_state = view_state;
	}

	pub fn sync_buffer_config(&mut self, font_system: &mut FontSystem, text: &str, config: SceneConfig) -> bool {
		if self.config == config {
			return false;
		}

		let width_only_change = self.width_only_config_change(config);
		self.clear_snapshot();
		self.config = config;
		if width_only_change {
			self.resize_buffer(font_system, self.config.max_width);
		} else {
			self.replace_buffer(font_system, text);
		}
		true
	}

	pub fn sync_buffer_width(&mut self, font_system: &mut FontSystem, width: f32) -> bool {
		if (self.config.max_width - width).abs() < 0.5 {
			return false;
		}

		self.clear_snapshot();
		self.resize_buffer(font_system, width);
		self.config.max_width = width;
		true
	}

	pub fn reset(&mut self, font_system: &mut FontSystem, text: &str, config: SceneConfig) {
		self.config = config;
		self.replace_buffer(font_system, text);
		self.clear_snapshot();
		self.view_state = EditorViewState::default();
	}

	pub fn document_layout(&self, text: &str) -> DocumentLayout {
		let font_names = self.font_names.borrow();
		DocumentLayout::build(text, &self.buffer, self.config, font_names.as_ref())
	}

	pub fn cached_document_layout_arc(&self) -> Option<Arc<DocumentLayout>> {
		self.document_layout.borrow().as_ref().map(Arc::clone)
	}

	pub fn set_document_layout(&self, document_layout: Arc<DocumentLayout>) {
		self.viewport_metrics.set(Some(EditorViewportMetrics {
			wrapping: self.config.wrapping,
			measured_width: document_layout.measured_width,
			measured_height: document_layout.measured_height,
		}));
		self.document_layout.replace(Some(document_layout));
	}

	pub fn viewport_metrics(&self) -> EditorViewportMetrics {
		self.viewport_metrics.get().unwrap_or_else(|| {
			let (measured_width, measured_height) = measure_buffer(&self.buffer);
			let metrics = EditorViewportMetrics {
				wrapping: self.config.wrapping,
				measured_width,
				measured_height,
			};
			self.viewport_metrics.set(Some(metrics));
			metrics
		})
	}

	pub fn text_layer_state(&self) -> EditorTextLayerState {
		let metrics = self.viewport_metrics();
		EditorTextLayerState {
			buffer: Arc::downgrade(&self.buffer),
			measured_height: metrics.measured_height,
		}
	}

	pub fn hit(&self, point: Point) -> Option<Cursor> {
		self.buffer.hit(point.x, point.y)
	}

	pub fn insert_cursor_rectangle(&self, text: &str, byte: usize) -> Option<LayoutRect> {
		insert_cursor_rectangle(&self.buffer, self.config.font_size, text, byte)
	}

	pub fn insert_cursor_block(&self, text: &str, byte: usize) -> Option<LayoutRect> {
		insert_cursor_block(&self.buffer, self.config.font_size, text, byte)
	}

	pub fn rebuild_buffer(&mut self, font_system: &mut FontSystem, text: &str, edit: &TextEdit) {
		let started = Instant::now();
		self.clear_snapshot();
		self.replace_buffer(font_system, text);
		debug!(
			duration_ms = duration_ms(started.elapsed()),
			text_bytes = text.len(),
			inserted_bytes = edit.inserted.len(),
			range_start = edit.range.start,
			range_end = edit.range.end,
			"layout edit rebuilt full buffer"
		);
	}

	pub fn apply_incremental_edit(&mut self, font_system: &mut FontSystem, text: &str, edit: &TextEdit) {
		let started = Instant::now();
		self.clear_snapshot();
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
		self.font_names
			.replace(resolve_font_names_from_buffer(font_system, buffer));
		trace!(
			duration_ms = duration_ms(started.elapsed()),
			text_bytes = text.len(),
			inserted_bytes = edit.inserted.len(),
			range_start = edit.range.start,
			range_end = edit.range.end,
			"layout edit updated buffer"
		);
	}

	#[cfg(test)]
	pub fn buffer_text(&self) -> String {
		self.buffer.lines.iter().fold(String::new(), |mut text, line| {
			text.push_str(line.text());
			text.push_str(line.ending().as_str());
			text
		})
	}

	fn width_only_config_change(&self, config: SceneConfig) -> bool {
		self.config.font_choice == config.font_choice
			&& self.config.shaping == config.shaping
			&& self.config.wrapping == config.wrapping
			&& (self.config.font_size - config.font_size).abs() < f32::EPSILON
			&& (self.config.line_height - config.line_height).abs() < f32::EPSILON
	}

	fn resize_buffer(&mut self, font_system: &mut FontSystem, width: f32) {
		let buffer = Arc::make_mut(&mut self.buffer);
		buffer.set_size(font_system, Some(width), None);
	}

	fn replace_buffer(&mut self, font_system: &mut FontSystem, text: &str) {
		self.buffer = Arc::new(build_buffer(font_system, text, self.config));
		self.font_names
			.replace(resolve_font_names_from_buffer(font_system, &self.buffer));
	}

	fn clear_snapshot(&self) {
		self.document_layout.replace(None);
		self.viewport_metrics.set(None);
	}
}

pub fn edit_changes_line_structure(text: &str, edit: &TextEdit) -> bool {
	// This editor's hard-line model treats only `\n` as a structural line break.
	text.get(edit.range.clone())
		.is_some_and(|removed| removed.contains('\n'))
		|| edit.inserted.contains('\n')
}

fn measure_buffer(buffer: &Buffer) -> (f32, f32) {
	buffer.layout_runs().fold((0.0, 0.0), |(width, height), run| {
		(width.max(run.line_w), height.max(run.line_top + run.line_height))
	})
}
