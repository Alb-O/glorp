use {
	super::text::byte_to_cursor,
	crate::{
		overlay::LayoutRect,
		scene::{DocumentLayout, LayoutCluster, line_byte_offsets},
	},
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

pub(super) fn cluster_rectangle(cluster: &LayoutCluster) -> LayoutRect {
	LayoutRect {
		x: cluster.x,
		y: cluster.y,
		width: cluster.width.max(1.0),
		height: cluster.height.max(1.0),
	}
}

pub(super) fn span_rectangle(start: &LayoutCluster, end: &LayoutCluster) -> LayoutRect {
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
	// Insert-mode hit tracking needs both the line lookup and the byte offset;
	// compute the offsets once and reuse them for both.
	let line_offsets = line_byte_offsets(text);
	let cursor = byte_to_cursor_with_offsets(text, &line_offsets, byte);
	let line = buffer.lines.get(cursor.line)?;
	let layout = line.layout_opt()?;
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

pub(super) fn selection_rectangles(layout: &DocumentLayout, range: &Range<usize>) -> Arc<[LayoutRect]> {
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
		let continues_span =
			cluster.run_index == span_end.run_index && cluster.byte_range.start <= span_end.byte_range.end;
		if !continues_span {
			rectangles.push(span_rectangle(span_start, span_end));
			span_start = cluster;
		}
		span_end = cluster;
	}

	rectangles.push(span_rectangle(span_start, span_end));
	rectangles.into()
}

pub(super) fn normal_selection_geometry(
	buffer: &Buffer, text: &str, range: &Range<usize>, active_byte: usize,
) -> (Arc<[LayoutRect]>, Option<LayoutRect>) {
	let mut rectangles = Vec::new();
	let mut span: Option<(usize, usize, LayoutRect)> = None;
	let mut next_target: Option<(usize, usize, LayoutRect)> = None;
	let mut previous_target: Option<(usize, usize, LayoutRect)> = None;

	for_each_cluster_rect(buffer, text, |run_index, byte_range, rect| {
		if byte_range.end > range.start && byte_range.start < range.end {
			match span.as_mut() {
				Some((span_run_index, span_end, span_rect))
					if *span_run_index == run_index && byte_range.start <= *span_end =>
				{
					*span_end = (*span_end).max(byte_range.end);
					merge_rect(span_rect, rect);
				}
				Some(_) => {
					push_span(&mut rectangles, &mut span);
					span = Some((run_index, byte_range.end, rect));
				}
				None => {
					span = Some((run_index, byte_range.end, rect));
				}
			}
		}

		if byte_range.end > active_byte
			&& next_target.as_ref().is_none_or(|(best_start, best_end, _)| {
				byte_range.start < *best_start || (byte_range.start == *best_start && byte_range.end < *best_end)
			}) {
			next_target = Some((byte_range.start, byte_range.end, rect));
		}

		if byte_range.start <= active_byte
			&& previous_target.as_ref().is_none_or(|(best_start, best_end, _)| {
				byte_range.start > *best_start || (byte_range.start == *best_start && byte_range.end > *best_end)
			}) {
			previous_target = Some((byte_range.start, byte_range.end, rect));
		}
	});

	push_span(&mut rectangles, &mut span);

	(
		rectangles.into(),
		next_target
			.map(|(_, _, rect)| rect)
			.or_else(|| previous_target.map(|(_, _, rect)| rect)),
	)
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
	let y = (visual_lines_offset(cursor.line, buffer) + visual_line as f32) * line_height - scroll.vertical;

	Some(InsertCursorGeometry {
		x: offset,
		y,
		height: line_height,
		block_width,
	})
}

fn for_each_cluster_rect(buffer: &Buffer, text: &str, mut f: impl FnMut(usize, Range<usize>, LayoutRect)) {
	let line_byte_offsets = line_byte_offsets(text);

	for (run_index, run) in buffer.layout_runs().enumerate() {
		let line_byte_offset = line_byte_offsets[run.line_i];
		let mut current: Option<(Range<usize>, LayoutRect)> = None;

		for glyph in run.glyphs {
			let byte_range = (line_byte_offset + glyph.start)..(line_byte_offset + glyph.end);
			let glyph_y = run.line_top + glyph.y;
			let glyph_height = glyph.line_height_opt.unwrap_or(run.line_height).max(1.0);

			match current.as_mut() {
				Some((current_range, current_rect)) if *current_range == byte_range => {
					current_rect.width = (glyph.x + glyph.w - current_rect.x).max(current_rect.width);
					current_rect.height = current_rect.height.max(glyph_height);
					current_rect.y = current_rect.y.min(glyph_y);
				}
				_ => {
					if let Some((byte_range, rect)) = current.replace((
						byte_range,
						LayoutRect {
							x: glyph.x,
							y: glyph_y,
							width: glyph.w.max(1.0),
							height: glyph_height,
						},
					)) {
						f(run_index, byte_range, rect);
					}
				}
			}
		}

		current
			.into_iter()
			.for_each(|(byte_range, rect)| f(run_index, byte_range, rect));
	}
}

fn merge_rect(target: &mut LayoutRect, next: LayoutRect) {
	let left = target.x.min(next.x);
	let right = (target.x + target.width).max(next.x + next.width);
	target.x = left;
	target.y = target.y.min(next.y);
	target.width = (right - left).max(1.0);
	target.height = target.height.max(next.height);
}

fn push_span(rectangles: &mut Vec<LayoutRect>, span: &mut Option<(usize, usize, LayoutRect)>) {
	if let Some((_, _, rect)) = span.take() {
		rectangles.push(rect);
	}
}

fn visual_lines_offset(line: usize, buffer: &Buffer) -> f32 {
	let scroll = buffer.scroll();
	let start = scroll.line.min(line);
	let end = scroll.line.max(line);
	let visual_lines = buffer.lines[start..end].iter().map(visual_line_len).sum::<f32>();

	if scroll.line < line {
		visual_lines
	} else {
		-visual_lines
	}
}

fn visual_line_len(line: &cosmic_text::BufferLine) -> f32 {
	line.layout_opt().map_or(0.0, |layout| layout.len() as f32)
}

fn byte_to_cursor_with_offsets(text: &str, line_offsets: &[usize], byte: usize) -> cosmic_text::Cursor {
	let mut clamped = byte.min(text.len());
	while clamped > 0 && !text.is_char_boundary(clamped) {
		clamped -= 1;
	}

	// `partition_point` keeps this in the same offset-space as the caller's
	// reused line table instead of rebuilding line boundaries through `byte_to_cursor`.
	let line = line_offsets
		.partition_point(|offset| *offset <= clamped)
		.saturating_sub(1);
	let index = clamped - line_offsets[line];
	cosmic_text::Cursor::new(line, index)
}
