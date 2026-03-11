use iced::advanced::text::{Alignment, LineHeight};
use iced::alignment;
use iced::widget::canvas;
use iced::{Color, Font, Pixels, Point, Rectangle, Size, Theme, mouse};

use crate::scene::{LayoutScene, PathCommand};
use crate::types::{CanvasTarget, Message};

#[derive(Debug, Clone)]
pub(crate) struct GlyphCanvas {
	pub(crate) scene: LayoutScene,
	pub(crate) show_baselines: bool,
	pub(crate) show_hitboxes: bool,
	pub(crate) hovered_target: Option<CanvasTarget>,
	pub(crate) selected_target: Option<CanvasTarget>,
}

#[derive(Debug, Default)]
pub(crate) struct CanvasState {
	hovered_target: Option<CanvasTarget>,
}

impl canvas::Program<Message> for GlyphCanvas {
	type State = CanvasState;

	fn update(
		&self, state: &mut Self::State, event: &canvas::Event, bounds: Rectangle, cursor: mouse::Cursor,
	) -> Option<canvas::Action<Message>> {
		let cursor_target = cursor.position_in(bounds).and_then(|position| self.hit_test(position));

		match event {
			canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
				if state.hovered_target != cursor_target {
					state.hovered_target = cursor_target;
					return Some(canvas::Action::publish(Message::CanvasHovered(cursor_target)));
				}
			}
			canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
				if cursor.is_over(bounds) {
					state.hovered_target = cursor_target;
					return Some(canvas::Action::publish(Message::CanvasSelected(cursor_target)).and_capture());
				}
			}
			canvas::Event::Mouse(mouse::Event::ButtonPressed(_)) => {
				if cursor.is_over(bounds) {
					return Some(canvas::Action::capture());
				}
			}
			_ => {
				if !cursor.is_over(bounds) && state.hovered_target.is_some() {
					state.hovered_target = None;
					return Some(canvas::Action::publish(Message::CanvasHovered(None)));
				}
			}
		}

		None
	}

	fn draw(
		&self, _state: &Self::State, renderer: &iced::Renderer, _theme: &Theme, bounds: Rectangle,
		_cursor: iced::mouse::Cursor,
	) -> Vec<canvas::Geometry> {
		let mut frame = canvas::Frame::new(renderer, bounds.size());
		let origin = scene_origin();
		let text_area_top_left = origin;
		let text_area_size = Size::new(
			self.scene.max_width.max(1.0),
			self.scene.measured_height.max(bounds.height - origin.y - 24.0),
		);

		frame.fill_rectangle(Point::ORIGIN, bounds.size(), Color::from_rgb8(20, 24, 32));
		frame.fill_rectangle(text_area_top_left, text_area_size, Color::from_rgb8(28, 34, 46));
		frame.stroke_rectangle(
			text_area_top_left,
			Size::new(self.scene.max_width.max(1.0), self.scene.measured_height.max(1.0)),
			canvas::Stroke::default()
				.with_width(1.0)
				.with_color(Color::from_rgba(0.8, 0.8, 0.9, 0.65)),
		);

		let guide = canvas::Path::line(Point::new(origin.x, 0.0), Point::new(origin.x, bounds.height));
		frame.stroke(
			&guide,
			canvas::Stroke::default()
				.with_width(1.0)
				.with_color(Color::from_rgba(0.6, 0.7, 1.0, 0.18)),
		);

		if self.scene.draw_canvas_text {
			let max_width = if self.scene.canvas_wraps {
				self.scene.max_width
			} else {
				f32::INFINITY
			};

			frame.fill_text(canvas::Text {
				content: self.scene.text.clone(),
				position: origin,
				max_width,
				color: Color::from_rgba(0.4, 0.8, 1.0, 0.9),
				size: Pixels(self.scene.font_size),
				line_height: LineHeight::Absolute(Pixels(self.scene.line_height)),
				font: self.scene.font,
				align_x: Alignment::Left,
				align_y: alignment::Vertical::Top,
				shaping: self.scene.shaping.to_iced(),
			});
		}

		for (run_index, run) in self.scene.runs.iter().enumerate() {
			if self.selected_target == Some(CanvasTarget::Run(run_index)) {
				frame.fill_rectangle(
					Point::new(origin.x, origin.y + run.line_top),
					Size::new(
						self.scene.max_width.max(run.line_width).max(1.0),
						run.line_height.max(1.0),
					),
					Color::from_rgba(1.0, 0.85, 0.2, 0.14),
				);
			} else if self.hovered_target == Some(CanvasTarget::Run(run_index)) {
				frame.fill_rectangle(
					Point::new(origin.x, origin.y + run.line_top),
					Size::new(
						self.scene.max_width.max(run.line_width).max(1.0),
						run.line_height.max(1.0),
					),
					Color::from_rgba(0.4, 0.8, 1.0, 0.1),
				);
			}

			if self.show_baselines {
				let top_line = canvas::Path::line(
					Point::new(origin.x, origin.y + run.line_top),
					Point::new(origin.x + self.scene.max_width, origin.y + run.line_top),
				);
				frame.stroke(
					&top_line,
					canvas::Stroke::default()
						.with_width(1.0)
						.with_color(Color::from_rgba(1.0, 0.6, 0.2, 0.45)),
				);

				let baseline = canvas::Path::line(
					Point::new(origin.x, origin.y + run.baseline),
					Point::new(origin.x + self.scene.max_width, origin.y + run.baseline),
				);
				frame.stroke(
					&baseline,
					canvas::Stroke::default()
						.with_width(1.0)
						.with_color(Color::from_rgba(0.4, 1.0, 0.6, 0.45)),
				);
			}

			for (glyph_index, glyph) in run.glyphs.iter().enumerate() {
				let target = CanvasTarget::Glyph { run_index, glyph_index };

				if self.selected_target == Some(target) {
					frame.fill_rectangle(
						Point::new(origin.x + glyph.x, origin.y + glyph.y),
						Size::new(glyph.width.max(1.0), glyph.height.max(1.0)),
						Color::from_rgba(1.0, 0.85, 0.2, 0.25),
					);
				} else if self.hovered_target == Some(target) {
					frame.fill_rectangle(
						Point::new(origin.x + glyph.x, origin.y + glyph.y),
						Size::new(glyph.width.max(1.0), glyph.height.max(1.0)),
						Color::from_rgba(0.4, 0.8, 1.0, 0.18),
					);
				}

				if self.show_hitboxes {
					frame.stroke_rectangle(
						Point::new(origin.x + glyph.x, origin.y + glyph.y),
						Size::new(glyph.width.max(0.5), glyph.height.max(0.5)),
						canvas::Stroke::default()
							.with_width(1.0)
							.with_color(if self.selected_target == Some(target) {
								Color::from_rgba(1.0, 0.9, 0.2, 0.95)
							} else if self.hovered_target == Some(target) {
								Color::from_rgba(0.5, 0.85, 1.0, 0.95)
							} else {
								Color::from_rgba(1.0, 0.3, 0.3, 0.6)
							}),
					);
				}

				if self.scene.draw_outlines {
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
			"runs={} glyphs={} fonts={} width={:.1} height={:.1}",
			self.scene.runs.len(),
			self.scene.glyph_count,
			self.scene.fonts_seen.len(),
			self.scene.measured_width,
			self.scene.measured_height,
		);
		frame.fill_text(canvas::Text {
			content: footer,
			position: Point::new(24.0, bounds.height - 24.0),
			color: Color::from_rgb8(180, 190, 210),
			size: Pixels(14.0),
			font: Font::MONOSPACE,
			..canvas::Text::default()
		});

		vec![frame.into_geometry()]
	}

	fn mouse_interaction(&self, _state: &Self::State, bounds: Rectangle, cursor: mouse::Cursor) -> mouse::Interaction {
		cursor
			.position_in(bounds)
			.and_then(|position| self.hit_test(position))
			.map(|_| mouse::Interaction::Pointer)
			.unwrap_or_default()
	}
}

impl GlyphCanvas {
	fn hit_test(&self, cursor_position: Point) -> Option<CanvasTarget> {
		let local = Point::new(
			cursor_position.x - scene_origin().x,
			cursor_position.y - scene_origin().y,
		);
		self.scene.hit_test(local)
	}
}

fn scene_origin() -> Point {
	Point::new(24.0, 28.0)
}
