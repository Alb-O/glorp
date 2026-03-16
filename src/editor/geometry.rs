use {
	super::{
		layout::{BufferClusterInfo, BufferLayoutSnapshot},
		text::byte_to_cursor,
	},
	crate::{overlay::LayoutRect, scene::line_byte_offsets},
	cosmic_text::Buffer,
	std::{ops::Range, sync::Arc},
};

#[derive(Debug, Clone, Copy)]
struct InsertCursorGeometry {
	x: f32,
	y: f32,
	height: f32,
	block_width: f32,
}

pub(super) fn cluster_rectangle(cluster: &BufferClusterInfo) -> LayoutRect {
	LayoutRect {
		x: cluster.x,
		y: cluster.y,
		width: cluster.width.max(1.0),
		height: cluster.height.max(1.0),
	}
}

pub(super) fn span_rectangle(start: &BufferClusterInfo, end: &BufferClusterInfo) -> LayoutRect {
	let left = start.x.min(end.x);
	let right = (start.x + start.width).max(end.x + end.width);

	LayoutRect {
		x: left,
		y: start.y.min(end.y),
		width: (right - left).max(1.0),
		height: start.height.max(end.height),
	}
}

pub(super) fn insert_cursor_rectangle(buffer: &Buffer, font_size: f32, text: &str, byte: usize) -> Option<LayoutRect> {
	let geometry = insert_cursor_geometry(buffer, font_size, text, byte)?;

	Some(LayoutRect {
		x: geometry.x,
		y: geometry.y,
		width: 2.0,
		height: geometry.height,
	})
}

pub(super) fn insert_cursor_block(buffer: &Buffer, font_size: f32, text: &str, byte: usize) -> Option<LayoutRect> {
	let geometry = insert_cursor_geometry(buffer, font_size, text, byte)?;

	Some(LayoutRect {
		x: geometry.x,
		y: geometry.y,
		width: geometry.block_width.max(2.0),
		height: geometry.height,
	})
}

pub(super) fn insert_selection_range(buffer: &Buffer, text: &str, byte: usize) -> Option<Range<usize>> {
	let cursor = byte_to_cursor(text, byte);
	let line = buffer.lines.get(cursor.line)?;
	let layout = line.layout_opt()?;
	let line_offsets = line_byte_offsets(text);
	let line_offset = *line_offsets.get(cursor.line)?;
	let ends_hard_line = byte
		.checked_add(1)
		.is_some_and(|next| line_offsets[1..].binary_search(&next).is_ok());
	let byte_limit = byte.saturating_add(1);
	let mut previous_cluster = None;

	for visual_line in layout {
		for glyph in &visual_line.glyphs {
			let cluster = (line_offset + glyph.start)..(line_offset + glyph.end);
			if previous_cluster.as_ref() == Some(&cluster) {
				continue;
			}

			if ends_hard_line {
				if cluster.start >= byte_limit {
					return previous_cluster;
				}
			} else if cluster.end > byte {
				return Some(cluster);
			}

			previous_cluster = Some(cluster);
		}
	}

	previous_cluster
}

pub(super) fn selection_rectangles(layout: &BufferLayoutSnapshot, range: &Range<usize>) -> Arc<[LayoutRect]> {
	let mut rectangles = Vec::new();
	let mut selected = layout
		.clusters()
		.iter()
		.filter(|cluster| cluster.byte_range.end > range.start && cluster.byte_range.start < range.end);
	let Some(mut span_start) = selected.next() else {
		return Arc::from([]);
	};
	let mut span_end = span_start;

	for cluster in selected {
		if cluster.run_index == span_end.run_index && cluster.byte_range.start <= span_end.byte_range.end {
			span_end = cluster;
		} else {
			rectangles.push(span_rectangle(span_start, span_end));
			span_start = cluster;
			span_end = cluster;
		}
	}

	rectangles.push(span_rectangle(span_start, span_end));
	rectangles.into()
}

fn insert_cursor_geometry(buffer: &Buffer, font_size: f32, text: &str, byte: usize) -> Option<InsertCursorGeometry> {
	let cursor = byte_to_cursor(text, byte);
	let line_height = buffer.metrics().line_height.max(1.0);
	let default_width = (font_size * 0.6).max(2.0);
	let scroll = buffer.scroll();
	let line = buffer.lines.get(cursor.line)?;
	let layout = line.layout_opt()?;

	let (visual_line, offset, block_width) = layout
		.iter()
		.enumerate()
		.find_map(|(index, line)| {
			let start = line.glyphs.first().map_or(0, |glyph| glyph.start);
			let end = line.glyphs.last().map_or(0, |glyph| glyph.end);
			let is_cursor_before_start = start > cursor.index;
			let is_cursor_before_end = cursor.index <= end;

			if is_cursor_before_start {
				index.checked_sub(1).map(|previous| {
					let previous_line = &layout[previous];
					let width = previous_line
						.glyphs
						.last()
						.map_or(default_width, |glyph| glyph.w.max(2.0));

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
					.map_or(default_width, |glyph| glyph.w.max(2.0));

				Some((index, offset, width))
			} else {
				None
			}
		})
		.unwrap_or_else(|| {
			let visual_line = layout.len().saturating_sub(1);
			let offset = layout.last().map_or(0.0, |line| line.w);
			let block_width = layout
				.last()
				.and_then(|line| line.glyphs.last())
				.map_or(default_width, |glyph| glyph.w.max(2.0));

			(visual_line, offset, block_width)
		});
	let y = (visual_lines_offset(cursor.line, buffer) + visual_line_count(visual_line)) * line_height - scroll.vertical;

	Some(InsertCursorGeometry {
		x: offset,
		y,
		height: line_height,
		block_width,
	})
}

fn visual_lines_offset(line: usize, buffer: &Buffer) -> f32 {
	let scroll = buffer.scroll();
	let start = scroll.line.min(line);
	let end = scroll.line.max(line);
	let visual_lines = buffer.lines[start..]
		.iter()
		.take(end - start)
		.map(visual_line_len)
		.sum::<f32>();

	if scroll.line < line {
		visual_lines
	} else {
		-visual_lines
	}
}

fn visual_line_len(line: &cosmic_text::BufferLine) -> f32 {
	line.layout_opt().map_or(0.0, |layout| layout.len() as f32)
}

fn visual_line_count(visual_line: usize) -> f32 {
	visual_line as f32
}
