use iced::widget::canvas;
use iced::{Color, Font, Pixels, Point, Rectangle, Size, Vector};

use std::sync::Arc;

use crate::overlay::{
	EditorOverlayTone, LayoutRect, OverlayLabelKind, OverlayPrimitive, OverlayRectKind, OverlaySpace,
};
use crate::scene::PathCommand;

use super::GlyphCanvas;
use super::geometry::{glyph_intersects_viewport, run_intersects_viewport, scrolled_origin, visible_scene_bounds};

pub(super) fn draw_static_scene(frame: &mut canvas::Frame, bounds: Rectangle, canvas: &GlyphCanvas, scroll: Vector) {
	let origin = scrolled_origin(scroll);
	let visible_scene_bounds = visible_scene_bounds(bounds, scroll);
	let inspect_runs = (canvas.show_hitboxes || canvas.scene.draw_outlines).then(|| canvas.scene.inspect_runs());
	for (run_index, run) in canvas.scene.runs.iter().enumerate() {
		if !run_intersects_viewport(run, visible_scene_bounds) {
			continue;
		}

		if canvas.show_baselines {
			let top_line = canvas::Path::line(
				Point::new(origin.x, origin.y + run.line_top),
				Point::new(origin.x + canvas.layout_width, origin.y + run.line_top),
			);
			frame.stroke(
				&top_line,
				canvas::Stroke::default()
					.with_width(1.0)
					.with_color(Color::from_rgba(1.0, 0.6, 0.2, 0.45)),
			);

			let baseline = canvas::Path::line(
				Point::new(origin.x, origin.y + run.baseline),
				Point::new(origin.x + canvas.layout_width, origin.y + run.baseline),
			);
			frame.stroke(
				&baseline,
				canvas::Stroke::default()
					.with_width(1.0)
					.with_color(Color::from_rgba(0.4, 1.0, 0.6, 0.45)),
			);
		}

		let Some(glyphs) = inspect_runs
			.as_ref()
			.and_then(|runs| runs.get(run_index))
			.map(|run| &run.glyphs)
		else {
			continue;
		};

		for glyph in glyphs {
			if !glyph_intersects_viewport(glyph, visible_scene_bounds) {
				continue;
			}

			if canvas.show_hitboxes {
				frame.stroke_rectangle(
					Point::new(origin.x + glyph.x, origin.y + glyph.y),
					Size::new(glyph.width.max(0.5), glyph.height.max(0.5)),
					canvas::Stroke::default()
						.with_width(1.0)
						.with_color(Color::from_rgba(1.0, 0.3, 0.3, 0.6)),
				);
			}

			if canvas.scene.draw_outlines {
				if let Some(outline) = &glyph.outline {
					let path = canvas::Path::new(|builder| {
						for command in &outline.commands {
							match command {
								PathCommand::MoveTo(point) => {
									builder.move_to(Point::new(origin.x + point.x, origin.y + point.y))
								}
								PathCommand::LineTo(point) => {
									builder.line_to(Point::new(origin.x + point.x, origin.y + point.y))
								}
								PathCommand::QuadTo(control, to) => builder.quadratic_curve_to(
									Point::new(origin.x + control.x, origin.y + control.y),
									Point::new(origin.x + to.x, origin.y + to.y),
								),
								PathCommand::CurveTo(a, b, to) => builder.bezier_curve_to(
									Point::new(origin.x + a.x, origin.y + a.y),
									Point::new(origin.x + b.x, origin.y + b.y),
									Point::new(origin.x + to.x, origin.y + to.y),
								),
								PathCommand::Close => builder.close(),
							}
						}
					});

					frame.fill(&path, Color::from_rgb8(245, 245, 240));
				} else {
					frame.fill_rectangle(
						Point::new(origin.x + glyph.x, origin.y + glyph.y),
						Size::new(glyph.width.max(1.0), glyph.height.max(1.0)),
						Color::from_rgba(0.95, 0.9, 0.3, 0.18),
					);
				}
			}
		}
	}
}

pub(super) fn draw_overlay(
	frame: &mut canvas::Frame, bounds: Rectangle, canvas: &GlyphCanvas, focused: bool, scroll: Vector,
) {
	let origin = scrolled_origin(scroll);

	for primitive in over_text_primitives(bounds, canvas, focused, scroll) {
		match primitive {
			OverlayPrimitive::Rect { rect, kind, space } => draw_rect_primitive(frame, origin, rect, kind, space),
			OverlayPrimitive::Label {
				position,
				kind,
				text,
				space,
			} => draw_label_primitive(frame, origin, position, kind, text, space),
		}
	}
}

pub(super) fn draw_underlay_overlay(
	frame: &mut canvas::Frame, editor: &crate::editor::EditorViewState, scroll: Vector,
) {
	let origin = scrolled_origin(scroll);
	for primitive in editor.overlays.iter().filter(is_under_text_primitive) {
		if let OverlayPrimitive::Rect { rect, kind, space } = primitive {
			draw_rect_primitive(frame, origin, *rect, *kind, *space);
		}
	}
}

fn over_text_primitives(
	bounds: Rectangle, canvas: &GlyphCanvas, focused: bool, scroll: Vector,
) -> Vec<OverlayPrimitive> {
	let mut overlays = Vec::with_capacity(canvas.inspect_overlays.len() + 3);
	overlays.extend(canvas.inspect_overlays.iter().cloned());

	if focused {
		overlays.push(OverlayPrimitive::scene_rect(
			LayoutRect {
				x: 0.0,
				y: 0.0,
				width: canvas.layout_width.max(1.0),
				height: canvas.scene.measured_height.max(1.0),
			},
			OverlayRectKind::EditorFocusFrame(EditorOverlayTone::from(canvas.editor.mode)),
		));
	}

	overlays.push(OverlayPrimitive::viewport_label(
		Point::new(24.0, bounds.height - 24.0),
		OverlayLabelKind::SceneFooter,
		format!(
			"runs={} glyphs={} clusters={} fonts={} width={:.1} height={:.1}",
			canvas.scene.runs.len(),
			canvas.scene.glyph_count,
			canvas.scene.clusters().len(),
			canvas.scene.font_count,
			canvas.scene.measured_width,
			canvas.scene.measured_height,
		),
	));
	overlays.push(OverlayPrimitive::viewport_label(
		Point::new(bounds.width - 170.0, bounds.height - 24.0),
		OverlayLabelKind::CanvasStatus,
		format!(
			"mode={} focus={focused} scroll={:.0},{:.0}",
			canvas.editor.mode, scroll.x, scroll.y
		),
	));

	overlays
}

fn is_under_text_primitive(primitive: &&OverlayPrimitive) -> bool {
	matches!(
		primitive,
		OverlayPrimitive::Rect {
			kind: OverlayRectKind::EditorSelection(_)
				| OverlayRectKind::EditorActive(_)
				| OverlayRectKind::EditorInsertBlock(_)
				| OverlayRectKind::EditorCaret(_),
			..
		}
	)
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
	frame: &mut canvas::Frame, origin: Point, rect: LayoutRect, kind: OverlayRectKind, space: OverlaySpace,
) {
	let point = rect_origin(origin, rect, space);
	let size = Size::new(rect.width.max(1.0), rect.height.max(1.0));
	let style = rect_style(kind);

	if let Some(fill) = style.fill {
		frame.fill_rectangle(point, size, fill);
	}

	if let Some((stroke, stroke_width)) = style.stroke {
		frame.stroke_rectangle(
			point,
			size,
			canvas::Stroke::default().with_width(stroke_width).with_color(stroke),
		);
	}
}

fn draw_label_primitive(
	frame: &mut canvas::Frame, origin: Point, position: Point, kind: OverlayLabelKind, text: Arc<str>,
	space: OverlaySpace,
) {
	let position = point_in_space(origin, position, space);
	let style = label_style(kind);
	frame.fill_text(canvas::Text {
		content: text.to_string(),
		position,
		color: style.color,
		size: style.size,
		font: style.font,
		..canvas::Text::default()
	});
}

fn rect_origin(origin: Point, rect: LayoutRect, space: OverlaySpace) -> Point {
	match space {
		OverlaySpace::Scene => Point::new(origin.x + rect.x, origin.y + rect.y),
		OverlaySpace::Viewport => Point::new(rect.x, rect.y),
	}
}

fn point_in_space(origin: Point, position: Point, space: OverlaySpace) -> Point {
	match space {
		OverlaySpace::Scene => Point::new(origin.x + position.x, origin.y + position.y),
		OverlaySpace::Viewport => position,
	}
}

fn rect_style(kind: OverlayRectKind) -> RectStyle {
	match kind {
		OverlayRectKind::EditorSelection(tone) => {
			let palette = selection_palette(tone);
			RectStyle {
				fill: Some(palette.selection_fill),
				stroke: Some((palette.selection_stroke, 1.0)),
			}
		}
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
