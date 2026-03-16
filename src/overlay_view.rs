use {
	crate::{
		canvas_view::scene_origin,
		editor::EditorViewState,
		overlay::{
			EditorOverlayTone, LayoutRect, OverlayLabelKind, OverlayLayer, OverlayPrimitive, OverlayRectKind,
			OverlaySpace,
		},
		perf::CanvasPerfSink,
		scene::LayoutScene,
		types::Message,
	},
	iced::{
		Color, Element, Font, Length, Pixels, Point, Rectangle, Size, Theme, Vector,
		advanced::{
			Layout, Renderer as _, Widget, layout, mouse, renderer,
			text::{LineHeight, Renderer as _, Shaping, Text, Wrapping},
			widget::Tree,
		},
		alignment,
	},
	std::{cmp::Ordering, sync::Arc, time::Instant},
};

#[derive(Debug, Clone)]
pub(crate) struct SceneOverlayLayer {
	scene: LayoutScene,
	layout_width: f32,
	inspect_overlays: Arc<[OverlayPrimitive]>,
	editor: EditorViewState,
	focused: bool,
	scroll: Vector,
	perf: CanvasPerfSink,
	width: Length,
	height: Length,
}

impl SceneOverlayLayer {
	pub(crate) fn new(
		scene: LayoutScene, layout_width: f32, inspect_overlays: Arc<[OverlayPrimitive]>, editor: EditorViewState,
		focused: bool, scroll: Vector, perf: CanvasPerfSink,
	) -> Self {
		Self {
			scene,
			layout_width,
			inspect_overlays,
			editor,
			focused,
			scroll,
			perf,
			width: Length::Fill,
			height: Length::Fill,
		}
	}

	pub(crate) fn width(mut self, width: impl Into<Length>) -> Self {
		self.width = width.into();
		self
	}

	pub(crate) fn height(mut self, height: impl Into<Length>) -> Self {
		self.height = height.into();
		self
	}
}

#[derive(Debug, Clone)]
pub(crate) struct EditorUnderlayLayer {
	editor: EditorViewState,
	scroll: Vector,
	perf: CanvasPerfSink,
	width: Length,
	height: Length,
}

impl EditorUnderlayLayer {
	pub(crate) fn new(editor: EditorViewState, scroll: Vector, perf: CanvasPerfSink) -> Self {
		Self {
			editor,
			scroll,
			perf,
			width: Length::Fill,
			height: Length::Fill,
		}
	}

	pub(crate) fn width(mut self, width: impl Into<Length>) -> Self {
		self.width = width.into();
		self
	}

	pub(crate) fn height(mut self, height: impl Into<Length>) -> Self {
		self.height = height.into();
		self
	}
}

impl Widget<Message, Theme, iced::Renderer> for SceneOverlayLayer {
	fn size(&self) -> Size<Length> {
		Size::new(self.width, self.height)
	}

	fn layout(&mut self, _tree: &mut Tree, _renderer: &iced::Renderer, limits: &layout::Limits) -> layout::Node {
		layout::Node::new(limits.resolve(
			self.width,
			self.height,
			Size::new(self.layout_width.max(1.0), self.scene.measured_height.max(1.0)),
		))
	}

	fn draw(
		&self, _tree: &Tree, renderer: &mut iced::Renderer, _theme: &Theme, _style: &renderer::Style,
		layout: Layout<'_>, _cursor: mouse::Cursor, _viewport: &Rectangle,
	) {
		let started = Instant::now();
		let bounds = layout.bounds();
		let origin = Point::new(
			bounds.x + scene_origin().x - self.scroll.x,
			bounds.y + scene_origin().y - self.scroll.y,
		);

		for primitive in overlay_primitives(bounds, self) {
			match primitive {
				OverlayPrimitive::Rect {
					rect,
					kind,
					space,
					layer: OverlayLayer::OverText,
				} => draw_rect_primitive(renderer, bounds, origin, rect, kind, space),
				OverlayPrimitive::Label {
					position,
					kind,
					text,
					space,
					layer: OverlayLayer::OverText,
				} => draw_label_primitive(renderer, bounds, origin, position, kind, &text, space),
				_ => {}
			}
		}

		self.perf.record_canvas_overlay(started.elapsed());
	}
}

impl Widget<Message, Theme, iced::Renderer> for EditorUnderlayLayer {
	fn size(&self) -> Size<Length> {
		Size::new(self.width, self.height)
	}

	fn layout(&mut self, _tree: &mut Tree, _renderer: &iced::Renderer, limits: &layout::Limits) -> layout::Node {
		layout::Node::new(limits.resolve(self.width, self.height, Size::ZERO))
	}

	fn draw(
		&self, _tree: &Tree, renderer: &mut iced::Renderer, _theme: &Theme, _style: &renderer::Style,
		layout: Layout<'_>, _cursor: mouse::Cursor, _viewport: &Rectangle,
	) {
		let started = Instant::now();
		let bounds = layout.bounds();
		let origin = Point::new(
			bounds.x + scene_origin().x - self.scroll.x,
			bounds.y + scene_origin().y - self.scroll.y,
		);

		draw_selection_underlay(renderer, bounds, origin, &self.editor.overlays);

		for primitive in self
			.editor
			.overlays
			.iter()
			.filter(|primitive| primitive.layer() == OverlayLayer::UnderText)
		{
			if matches!(
				primitive,
				OverlayPrimitive::Rect {
					kind: OverlayRectKind::EditorSelection(_),
					..
				}
			) {
				continue;
			}

			if let OverlayPrimitive::Rect { rect, kind, space, .. } = primitive {
				draw_rect_primitive(renderer, bounds, origin, *rect, *kind, *space);
			}
		}

		self.perf.record_canvas_underlay(started.elapsed());
	}
}

impl From<SceneOverlayLayer> for Element<'_, Message> {
	fn from(widget: SceneOverlayLayer) -> Self {
		Element::new(widget)
	}
}

impl From<EditorUnderlayLayer> for Element<'_, Message> {
	fn from(widget: EditorUnderlayLayer) -> Self {
		Element::new(widget)
	}
}

fn overlay_primitives(bounds: Rectangle, overlay: &SceneOverlayLayer) -> Vec<OverlayPrimitive> {
	let mut primitives = Vec::with_capacity(overlay.inspect_overlays.len() + 3);
	primitives.extend(overlay.inspect_overlays.iter().cloned());

	if overlay.focused {
		primitives.push(OverlayPrimitive::scene_rect(
			LayoutRect {
				x: 0.0,
				y: 0.0,
				width: overlay.layout_width.max(1.0),
				height: overlay.scene.measured_height.max(1.0),
			},
			OverlayRectKind::EditorFocusFrame(EditorOverlayTone::from(overlay.editor.mode)),
			OverlayLayer::OverText,
		));
	}

	primitives.push(OverlayPrimitive::viewport_label(
		Point::new(24.0, bounds.height - 24.0),
		OverlayLabelKind::SceneFooter,
		format!(
			"runs={} glyphs={} clusters={} fonts={} width={:.1} height={:.1}",
			overlay.scene.runs.len(),
			overlay.scene.glyph_count,
			overlay.scene.clusters().len(),
			overlay.scene.font_count,
			overlay.scene.measured_width,
			overlay.scene.measured_height,
		),
		OverlayLayer::OverText,
	));
	primitives.push(OverlayPrimitive::viewport_label(
		Point::new(bounds.width - 170.0, bounds.height - 24.0),
		OverlayLabelKind::CanvasStatus,
		format!(
			"mode={} focus={} scroll={:.0},{:.0}",
			overlay.editor.mode, overlay.focused, overlay.scroll.x, overlay.scroll.y
		),
		OverlayLayer::OverText,
	));

	primitives
}

#[derive(Debug, Clone, Copy)]
struct RectStyle {
	fill: Option<Color>,
	stroke: Option<(Color, f32)>,
}

#[derive(Debug, Clone, Copy)]
struct LabelStyle {
	color: Color,
	size: Pixels,
	font: Font,
}

#[derive(Debug, Clone, Copy)]
struct SelectionPalette {
	selection_fill: Color,
	selection_stroke: Color,
	active_fill: Color,
	active_stroke: Color,
	caret_fill: Color,
	focus_stroke: Color,
}

fn draw_rect_primitive(
	renderer: &mut iced::Renderer, bounds: Rectangle, origin: Point, rect: LayoutRect, kind: OverlayRectKind,
	space: OverlaySpace,
) {
	let rect_bounds = rect_bounds(bounds, origin, rect, space);
	let style = rect_style(kind);

	renderer.fill_quad(
		renderer::Quad {
			bounds: rect_bounds,
			border: style
				.stroke
				.map_or_else(iced::Border::default, |(color, width)| iced::Border {
					color,
					width,
					..iced::Border::default()
				}),
			..renderer::Quad::default()
		},
		style.fill.unwrap_or(Color::TRANSPARENT),
	);
}

fn draw_label_primitive(
	renderer: &mut iced::Renderer, bounds: Rectangle, origin: Point, position: Point, kind: OverlayLabelKind,
	text: &str, space: OverlaySpace,
) {
	let style = label_style(kind);
	renderer.fill_text(
		Text {
			content: text.to_string(),
			bounds: Size::new(f32::INFINITY, style.size.0),
			size: style.size,
			line_height: LineHeight::Absolute(style.size),
			font: style.font,
			align_x: iced::advanced::text::Alignment::Left,
			align_y: alignment::Vertical::Top,
			shaping: Shaping::Basic,
			wrapping: Wrapping::None,
			ellipsis: iced::advanced::text::Ellipsis::None,
			hint_factor: None,
		},
		point_in_space(bounds, origin, position, space),
		style.color,
		bounds,
	);
}

fn rect_bounds(bounds: Rectangle, origin: Point, rect: LayoutRect, space: OverlaySpace) -> Rectangle {
	Rectangle::new(
		point_in_space(bounds, origin, Point::new(rect.x, rect.y), space),
		Size::new(rect.width.max(1.0), rect.height.max(1.0)),
	)
}

fn point_in_space(bounds: Rectangle, origin: Point, position: Point, space: OverlaySpace) -> Point {
	match space {
		OverlaySpace::Scene => Point::new(origin.x + position.x, origin.y + position.y),
		OverlaySpace::Viewport => Point::new(bounds.x + position.x, bounds.y + position.y),
	}
}

fn draw_selection_underlay(
	renderer: &mut iced::Renderer, bounds: Rectangle, origin: Point, overlays: &[OverlayPrimitive],
) {
	let mut normal = Vec::new();
	let mut insert = Vec::new();

	for primitive in overlays
		.iter()
		.filter(|primitive| primitive.layer() == OverlayLayer::UnderText)
	{
		let OverlayPrimitive::Rect {
			rect,
			kind: OverlayRectKind::EditorSelection(tone),
			space,
			..
		} = primitive
		else {
			continue;
		};

		let rect = rect_bounds(bounds, origin, *rect, *space);
		let rect = LayoutRect {
			x: rect.x,
			y: rect.y,
			width: rect.width.max(1.0),
			height: rect.height.max(1.0),
		};

		match tone {
			EditorOverlayTone::Normal => normal.push(rect),
			EditorOverlayTone::Insert => insert.push(rect),
		}
	}

	draw_selection_group(renderer, &normal, selection_palette(EditorOverlayTone::Normal));
	draw_selection_group(renderer, &insert, selection_palette(EditorOverlayTone::Insert));
}

fn draw_selection_group(renderer: &mut iced::Renderer, rectangles: &[LayoutRect], palette: SelectionPalette) {
	if rectangles.is_empty() {
		return;
	}

	for rect in rectangles {
		renderer.fill_quad(
			renderer::Quad {
				bounds: Rectangle::new(
					Point::new(rect.x, rect.y),
					Size::new(rect.width.max(1.0), rect.height.max(1.0)),
				),
				..renderer::Quad::default()
			},
			palette.selection_fill,
		);
	}

	for segment in merged_selection_outline(rectangles) {
		draw_outline_segment(renderer, segment, palette.selection_stroke);
	}
}

fn draw_outline_segment(renderer: &mut iced::Renderer, segment: OutlineSegment, color: Color) {
	let width = (segment.end.x - segment.start.x).abs().max(1.0);
	let height = (segment.end.y - segment.start.y).abs().max(1.0);
	let bounds = if width >= height {
		Rectangle::new(
			Point::new(segment.start.x.min(segment.end.x), segment.start.y.min(segment.end.y)),
			Size::new(width, 1.0),
		)
	} else {
		Rectangle::new(
			Point::new(segment.start.x.min(segment.end.x), segment.start.y.min(segment.end.y)),
			Size::new(1.0, height),
		)
	};

	renderer.fill_quad(
		renderer::Quad {
			bounds,
			..renderer::Quad::default()
		},
		color,
	);
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct OutlineSegment {
	start: Point,
	end: Point,
}

#[derive(Debug, Clone, Copy)]
struct HorizontalSegment {
	y: f32,
	x0: f32,
	x1: f32,
}

#[derive(Debug, Clone, Copy)]
struct VerticalSegment {
	x: f32,
	y0: f32,
	y1: f32,
}

fn merged_selection_outline(rectangles: &[LayoutRect]) -> Vec<OutlineSegment> {
	if rectangles.is_empty() {
		return Vec::new();
	}

	let xs = unique_sorted_edges(rectangles.iter().flat_map(|rect| [rect.x, rect.x + rect.width]));
	let ys = unique_sorted_edges(rectangles.iter().flat_map(|rect| [rect.y, rect.y + rect.height]));
	if xs.len() < 2 || ys.len() < 2 {
		return Vec::new();
	}

	let columns = xs.len() - 1;
	let rows = ys.len() - 1;
	let mut occupied = vec![false; rows * columns];

	for row in 0..rows {
		for column in 0..columns {
			let x0 = xs[column];
			let x1 = xs[column + 1];
			let y0 = ys[row];
			let y1 = ys[row + 1];
			occupied[row * columns + column] = rectangles
				.iter()
				.any(|rect| x0 >= rect.x && x1 <= rect.x + rect.width && y0 >= rect.y && y1 <= rect.y + rect.height);
		}
	}

	let mut horizontal = Vec::new();
	let mut vertical = Vec::new();

	for row in 0..rows {
		for column in 0..columns {
			if !occupied[row * columns + column] {
				continue;
			}

			if row == 0 || !occupied[(row - 1) * columns + column] {
				horizontal.push(HorizontalSegment {
					y: ys[row],
					x0: xs[column],
					x1: xs[column + 1],
				});
			}

			if row + 1 == rows || !occupied[(row + 1) * columns + column] {
				horizontal.push(HorizontalSegment {
					y: ys[row + 1],
					x0: xs[column],
					x1: xs[column + 1],
				});
			}

			if column == 0 || !occupied[row * columns + column - 1] {
				vertical.push(VerticalSegment {
					x: xs[column],
					y0: ys[row],
					y1: ys[row + 1],
				});
			}

			if column + 1 == columns || !occupied[row * columns + column + 1] {
				vertical.push(VerticalSegment {
					x: xs[column + 1],
					y0: ys[row],
					y1: ys[row + 1],
				});
			}
		}
	}

	merge_horizontal_segments(horizontal)
		.into_iter()
		.map(|segment| OutlineSegment {
			start: Point::new(segment.x0, segment.y),
			end: Point::new(segment.x1, segment.y),
		})
		.chain(
			merge_vertical_segments(vertical)
				.into_iter()
				.map(|segment| OutlineSegment {
					start: Point::new(segment.x, segment.y0),
					end: Point::new(segment.x, segment.y1),
				}),
		)
		.collect()
}

fn unique_sorted_edges(edges: impl IntoIterator<Item = f32>) -> Vec<f32> {
	let mut edges = edges.into_iter().collect::<Vec<_>>();
	edges.sort_by(f32::total_cmp);
	edges.dedup_by(|left, right| left.total_cmp(right) == Ordering::Equal);
	edges
}

fn merge_horizontal_segments(mut segments: Vec<HorizontalSegment>) -> Vec<HorizontalSegment> {
	segments.sort_by(|left, right| {
		left.y
			.total_cmp(&right.y)
			.then_with(|| left.x0.total_cmp(&right.x0))
			.then_with(|| left.x1.total_cmp(&right.x1))
	});
	merge_segments(
		segments,
		|left, right| left.y.total_cmp(&right.y) == Ordering::Equal && left.x1.total_cmp(&right.x0) != Ordering::Less,
		|left, right| HorizontalSegment {
			y: left.y,
			x0: left.x0,
			x1: left.x1.max(right.x1),
		},
	)
}

fn merge_vertical_segments(mut segments: Vec<VerticalSegment>) -> Vec<VerticalSegment> {
	segments.sort_by(|left, right| {
		left.x
			.total_cmp(&right.x)
			.then_with(|| left.y0.total_cmp(&right.y0))
			.then_with(|| left.y1.total_cmp(&right.y1))
	});
	merge_segments(
		segments,
		|left, right| left.x.total_cmp(&right.x) == Ordering::Equal && left.y1.total_cmp(&right.y0) != Ordering::Less,
		|left, right| VerticalSegment {
			x: left.x,
			y0: left.y0,
			y1: left.y1.max(right.y1),
		},
	)
}

fn merge_segments<T: Copy>(segments: Vec<T>, can_merge: impl Fn(&T, &T) -> bool, merge: impl Fn(T, T) -> T) -> Vec<T> {
	let mut merged = Vec::with_capacity(segments.len());
	for segment in segments {
		if let Some(last) = merged.last_mut()
			&& can_merge(last, &segment)
		{
			*last = merge(*last, segment);
			continue;
		}

		merged.push(segment);
	}
	merged
}

fn rect_style(kind: OverlayRectKind) -> RectStyle {
	match kind {
		OverlayRectKind::EditorSelection(_) => RectStyle {
			fill: Some(Color::from_rgba(0.28, 0.74, 1.0, 0.18)),
			stroke: Some((Color::from_rgba(0.6, 0.9, 1.0, 0.66), 1.0)),
		},
		OverlayRectKind::EditorActive(tone) => {
			let palette = selection_palette(tone);
			RectStyle {
				fill: Some(palette.active_fill),
				stroke: Some((palette.active_stroke, 1.5)),
			}
		}
		OverlayRectKind::EditorInsertBlock(tone) => {
			let palette = selection_palette(tone);
			RectStyle {
				fill: Some(palette.caret_fill),
				stroke: Some((palette.active_stroke, 1.5)),
			}
		}
		OverlayRectKind::EditorCaret(tone) => {
			let palette = selection_palette(tone);
			RectStyle {
				fill: Some(palette.caret_fill),
				stroke: None,
			}
		}
		OverlayRectKind::EditorFocusFrame(tone) => {
			let palette = selection_palette(tone);
			RectStyle {
				fill: None,
				stroke: Some((palette.focus_stroke, 1.0)),
			}
		}
		OverlayRectKind::InspectRunHover => RectStyle {
			fill: Some(Color::from_rgba(0.4, 0.8, 1.0, 0.1)),
			stroke: None,
		},
		OverlayRectKind::InspectRunSelected => RectStyle {
			fill: Some(Color::from_rgba(1.0, 0.85, 0.2, 0.14)),
			stroke: None,
		},
		OverlayRectKind::InspectGlyphHover => RectStyle {
			fill: Some(Color::from_rgba(0.4, 0.8, 1.0, 0.18)),
			stroke: None,
		},
		OverlayRectKind::InspectGlyphSelected => RectStyle {
			fill: Some(Color::from_rgba(1.0, 0.85, 0.2, 0.25)),
			stroke: None,
		},
		OverlayRectKind::InspectGlyphHitboxHover => RectStyle {
			fill: None,
			stroke: Some((Color::from_rgba(0.5, 0.85, 1.0, 0.95), 1.0)),
		},
		OverlayRectKind::InspectGlyphHitboxSelected => RectStyle {
			fill: None,
			stroke: Some((Color::from_rgba(1.0, 0.9, 0.2, 0.95), 1.0)),
		},
	}
}

fn label_style(kind: OverlayLabelKind) -> LabelStyle {
	match kind {
		OverlayLabelKind::SceneFooter => LabelStyle {
			color: Color::from_rgb8(180, 190, 210),
			size: Pixels(14.0),
			font: Font::MONOSPACE,
		},
		OverlayLabelKind::CanvasStatus => LabelStyle {
			color: Color::from_rgb8(210, 214, 228),
			size: Pixels(14.0),
			font: Font::MONOSPACE,
		},
	}
}

fn selection_palette(_tone: EditorOverlayTone) -> SelectionPalette {
	SelectionPalette {
		selection_fill: Color::from_rgba(0.28, 0.74, 1.0, 0.18),
		selection_stroke: Color::from_rgba(0.6, 0.9, 1.0, 0.66),
		active_fill: Color::from_rgba(0.1, 0.86, 0.72, 0.28),
		active_stroke: Color::from_rgba(0.66, 1.0, 0.9, 0.94),
		caret_fill: Color::from_rgba(0.62, 1.0, 0.88, 1.0),
		focus_stroke: Color::from_rgba(0.56, 0.94, 1.0, 0.84),
	}
}

#[cfg(test)]
mod tests {
	use {
		super::{OutlineSegment, merged_selection_outline},
		crate::overlay::LayoutRect,
		iced::Point,
	};

	#[test]
	fn merged_selection_outline_collapses_shared_row_edges() {
		let outline = merged_selection_outline(&[
			LayoutRect {
				x: 10.0,
				y: 20.0,
				width: 30.0,
				height: 8.0,
			},
			LayoutRect {
				x: 10.0,
				y: 28.0,
				width: 30.0,
				height: 8.0,
			},
		]);

		assert_eq!(
			outline,
			vec![
				OutlineSegment {
					start: Point::new(10.0, 20.0),
					end: Point::new(40.0, 20.0),
				},
				OutlineSegment {
					start: Point::new(10.0, 36.0),
					end: Point::new(40.0, 36.0),
				},
				OutlineSegment {
					start: Point::new(10.0, 20.0),
					end: Point::new(10.0, 36.0),
				},
				OutlineSegment {
					start: Point::new(40.0, 20.0),
					end: Point::new(40.0, 36.0),
				},
			]
		);
	}

	#[test]
	fn merged_selection_outline_keeps_outer_step_edges_only() {
		let outline = merged_selection_outline(&[
			LayoutRect {
				x: 10.0,
				y: 20.0,
				width: 40.0,
				height: 8.0,
			},
			LayoutRect {
				x: 10.0,
				y: 28.0,
				width: 20.0,
				height: 8.0,
			},
		]);

		assert_eq!(
			outline,
			vec![
				OutlineSegment {
					start: Point::new(10.0, 20.0),
					end: Point::new(50.0, 20.0),
				},
				OutlineSegment {
					start: Point::new(30.0, 28.0),
					end: Point::new(50.0, 28.0),
				},
				OutlineSegment {
					start: Point::new(10.0, 36.0),
					end: Point::new(30.0, 36.0),
				},
				OutlineSegment {
					start: Point::new(10.0, 20.0),
					end: Point::new(10.0, 36.0),
				},
				OutlineSegment {
					start: Point::new(30.0, 28.0),
					end: Point::new(30.0, 36.0),
				},
				OutlineSegment {
					start: Point::new(50.0, 20.0),
					end: Point::new(50.0, 28.0),
				},
			]
		);
	}
}
