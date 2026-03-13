use cosmic_text::Buffer;

use std::ops::Range;
use std::sync::Arc;

use crate::overlay::LayoutRect;

use super::layout::{BufferClusterInfo, BufferLayoutSnapshot};
use super::text::byte_to_cursor;

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

pub(super) fn selection_rectangles(layout: &BufferLayoutSnapshot, range: &Range<usize>) -> Arc<[LayoutRect]> {
	let selected = layout
		.clusters()
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

		rectangles.push(span_rectangle(span_start, span_end));
		span_start = cluster;
		span_end = cluster;
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
	let y = (visual_lines_offset(cursor.line, buffer) + visual_line as i32) as f32 * line_height - scroll.vertical;

	Some(InsertCursorGeometry {
		x: offset,
		y,
		height: line_height,
		block_width,
	})
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
