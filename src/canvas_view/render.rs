use {
	super::{
		GlyphCanvas,
		geometry::{glyph_intersects_viewport, run_intersects_viewport, scrolled_origin, visible_scene_bounds},
	},
	crate::scene::PathCommand,
	iced::{Color, Point, Rectangle, Size, Vector, widget::canvas},
};

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
