use iced::widget::canvas;
use iced::{Color, Font, Pixels, Point, Rectangle, Size, Vector};

use crate::scene::PathCommand;
use crate::types::CanvasTarget;

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

	let footer = format!(
		"runs={} glyphs={} clusters={} fonts={} width={:.1} height={:.1}",
		canvas.scene.runs.len(),
		canvas.scene.glyph_count,
		canvas.scene.clusters().len(),
		canvas.scene.font_count,
		canvas.scene.measured_width,
		canvas.scene.measured_height,
	);
	frame.fill_text(canvas::Text {
		content: footer,
		position: Point::new(24.0, bounds.height - 24.0),
		color: Color::from_rgb8(180, 190, 210),
		size: Pixels(14.0),
		font: Font::MONOSPACE,
		..canvas::Text::default()
	});
}

pub(super) fn draw_overlay(
	frame: &mut canvas::Frame, bounds: Rectangle, canvas: &GlyphCanvas, focused: bool, scroll: Vector,
) {
	let origin = scrolled_origin(scroll);

	for selection in canvas.editor.selection_rectangles.iter() {
		frame.fill_rectangle(
			Point::new(origin.x + selection.x, origin.y + selection.y),
			Size::new(selection.width.max(1.0), selection.height.max(1.0)),
			Color::from_rgba(1.0, 0.92, 0.45, 0.35),
		);
		frame.stroke_rectangle(
			Point::new(origin.x + selection.x, origin.y + selection.y),
			Size::new(selection.width.max(1.0), selection.height.max(1.0)),
			canvas::Stroke::default()
				.with_width(1.0)
				.with_color(Color::from_rgba(1.0, 0.95, 0.7, 0.92)),
		);
	}

	if canvas.show_inspector_overlays {
		if let Some(target) = canvas.hovered_target {
			draw_target_overlay(frame, canvas, origin, target, false);
		}

		if let Some(target) = canvas.selected_target {
			draw_target_overlay(frame, canvas, origin, target, true);
		}
	}

	if focused {
		frame.stroke_rectangle(
			origin,
			Size::new(canvas.layout_width.max(1.0), canvas.scene.measured_height.max(1.0)),
			canvas::Stroke::default()
				.with_width(1.0)
				.with_color(Color::from_rgba(0.96, 0.92, 0.7, 0.85)),
		);
	}

	frame.fill_text(canvas::Text {
		content: format!(
			"mode={} focus={focused} scroll={:.0},{:.0}",
			canvas.editor.mode, scroll.x, scroll.y
		),
		position: Point::new(bounds.width - 170.0, bounds.height - 24.0),
		color: Color::from_rgb8(210, 214, 228),
		size: Pixels(14.0),
		font: Font::MONOSPACE,
		..canvas::Text::default()
	});
}

fn draw_target_overlay(
	frame: &mut canvas::Frame, canvas: &GlyphCanvas, origin: Point, target: CanvasTarget, selected: bool,
) {
	match target {
		CanvasTarget::Run(run_index) => {
			let Some(run) = canvas.scene.runs.get(run_index) else {
				return;
			};

			frame.fill_rectangle(
				Point::new(origin.x, origin.y + run.line_top),
				Size::new(
					canvas.layout_width.max(run.line_width).max(1.0),
					run.line_height.max(1.0),
				),
				if selected {
					Color::from_rgba(1.0, 0.85, 0.2, 0.14)
				} else {
					Color::from_rgba(0.4, 0.8, 1.0, 0.1)
				},
			);
		}
		CanvasTarget::Glyph { run_index, glyph_index } => {
			let (glyph_origin, glyph_size) = if let Some(glyph) = canvas.scene.glyph(run_index, glyph_index) {
				(
					Point::new(origin.x + glyph.x, origin.y + glyph.y),
					Size::new(glyph.width.max(1.0), glyph.height.max(1.0)),
				)
			} else if let Some(cluster) = canvas
				.scene
				.cluster_index_for_target(target)
				.and_then(|index| canvas.scene.cluster(index))
			{
				(
					Point::new(origin.x + cluster.x, origin.y + cluster.y),
					Size::new(cluster.width.max(1.0), cluster.height.max(1.0)),
				)
			} else {
				return;
			};

			frame.fill_rectangle(
				glyph_origin,
				glyph_size,
				if selected {
					Color::from_rgba(1.0, 0.85, 0.2, 0.25)
				} else {
					Color::from_rgba(0.4, 0.8, 1.0, 0.18)
				},
			);

			if canvas.show_hitboxes {
				frame.stroke_rectangle(
					glyph_origin,
					Size::new(glyph_size.width.max(0.5), glyph_size.height.max(0.5)),
					canvas::Stroke::default().with_width(1.0).with_color(if selected {
						Color::from_rgba(1.0, 0.9, 0.2, 0.95)
					} else {
						Color::from_rgba(0.5, 0.85, 1.0, 0.95)
					}),
				);
			}
		}
	}
}
