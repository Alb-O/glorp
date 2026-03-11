use std::cell::Cell;

use iced::advanced::text::{Alignment, LineHeight};
use iced::alignment;
use iced::keyboard::{self, key};
use iced::widget::canvas;
use iced::{Color, Font, Pixels, Point, Rectangle, Size, Theme, mouse};

use crate::editor::{EditorCommand, EditorMode, EditorViewState};
use crate::scene::{LayoutScene, PathCommand};
use crate::types::{CanvasTarget, Message};

#[derive(Debug, Clone)]
pub(crate) struct GlyphCanvas {
	pub(crate) scene: LayoutScene,
	pub(crate) show_baselines: bool,
	pub(crate) show_hitboxes: bool,
	pub(crate) hovered_target: Option<CanvasTarget>,
	pub(crate) selected_target: Option<CanvasTarget>,
	pub(crate) editor: EditorViewState,
	pub(crate) scene_revision: u64,
}

#[derive(Debug, Default)]
pub(crate) struct CanvasState {
	hovered_target: Option<CanvasTarget>,
	focused: bool,
	scene_cache: canvas::Cache,
	cached_scene_revision: Cell<Option<u64>>,
}

impl canvas::Program<Message> for GlyphCanvas {
	type State = CanvasState;

	fn update(
		&self, state: &mut Self::State, event: &canvas::Event, bounds: Rectangle, cursor: mouse::Cursor,
	) -> Option<canvas::Action<Message>> {
		let cursor_position = cursor.position_in(bounds);
		let cursor_local = cursor_position.map(to_scene_local);
		let cursor_target = cursor_local.and_then(|position| self.scene.hit_test(position));

		match event {
			canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
				if state.hovered_target != cursor_target {
					state.hovered_target = cursor_target;
					return Some(canvas::Action::publish(Message::CanvasHovered(cursor_target)));
				}
			}
			canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
				if !cursor.is_over(bounds) {
					state.focused = false;
					return None;
				}

				let Some(position) = cursor_local else {
					return None;
				};

				state.focused = cursor.is_over(bounds);
				state.hovered_target = cursor_target;

				if cursor.is_over(bounds) {
					return Some(
						canvas::Action::publish(Message::CanvasClicked {
							target: cursor_target,
							position,
						})
						.and_capture(),
					);
				}
			}
			canvas::Event::Keyboard(keyboard::Event::KeyPressed {
				key,
				physical_key,
				modifiers,
				text,
				..
			}) if state.focused => {
				if let Some(command) = key_command(self.editor.mode, key, *physical_key, *modifiers, text.as_deref()) {
					return Some(canvas::Action::publish(Message::EditorCommand(command)).and_capture());
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
		&self, state: &Self::State, renderer: &iced::Renderer, _theme: &Theme, bounds: Rectangle,
		_cursor: iced::mouse::Cursor,
	) -> Vec<canvas::Geometry> {
		if state.cached_scene_revision.get() != Some(self.scene_revision) {
			state.scene_cache.clear();
			state.cached_scene_revision.set(Some(self.scene_revision));
		}

		let static_layer = state.scene_cache.draw(renderer, bounds.size(), |frame| {
			draw_static_scene(frame, bounds, self);
		});

		let mut overlay = canvas::Frame::new(renderer, bounds.size());
		draw_dynamic_overlay(&mut overlay, bounds, self, state.focused);

		vec![static_layer, overlay.into_geometry()]
	}

	fn mouse_interaction(&self, _state: &Self::State, bounds: Rectangle, cursor: mouse::Cursor) -> mouse::Interaction {
		cursor
			.position_in(bounds)
			.map(|_| mouse::Interaction::Text)
			.unwrap_or_default()
	}
}

fn draw_static_scene(frame: &mut canvas::Frame, bounds: Rectangle, canvas: &GlyphCanvas) {
	let origin = scene_origin();
	let text_area_size = Size::new(
		canvas.scene.max_width.max(1.0),
		canvas.scene.measured_height.max(bounds.height - origin.y - 24.0),
	);

	frame.fill_rectangle(Point::ORIGIN, bounds.size(), Color::from_rgb8(20, 24, 32));
	frame.fill_rectangle(origin, text_area_size, Color::from_rgb8(28, 34, 46));
	frame.stroke_rectangle(
		origin,
		Size::new(canvas.scene.max_width.max(1.0), canvas.scene.measured_height.max(1.0)),
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

	if canvas.scene.draw_canvas_text {
		let max_width = if canvas.scene.canvas_wraps {
			canvas.scene.max_width
		} else {
			f32::INFINITY
		};

		frame.fill_text(canvas::Text {
			content: canvas.scene.text.clone(),
			position: origin,
			max_width,
			color: Color::from_rgba(0.4, 0.8, 1.0, 0.9),
			size: Pixels(canvas.scene.font_size),
			line_height: LineHeight::Absolute(Pixels(canvas.scene.line_height)),
			font: canvas.scene.font,
			align_x: Alignment::Left,
			align_y: alignment::Vertical::Top,
			shaping: canvas.scene.shaping.to_iced(),
		});
	}

	for run in &canvas.scene.runs {
		if canvas.show_baselines {
			let top_line = canvas::Path::line(
				Point::new(origin.x, origin.y + run.line_top),
				Point::new(origin.x + canvas.scene.max_width, origin.y + run.line_top),
			);
			frame.stroke(
				&top_line,
				canvas::Stroke::default()
					.with_width(1.0)
					.with_color(Color::from_rgba(1.0, 0.6, 0.2, 0.45)),
			);

			let baseline = canvas::Path::line(
				Point::new(origin.x, origin.y + run.baseline),
				Point::new(origin.x + canvas.scene.max_width, origin.y + run.baseline),
			);
			frame.stroke(
				&baseline,
				canvas::Stroke::default()
					.with_width(1.0)
					.with_color(Color::from_rgba(0.4, 1.0, 0.6, 0.45)),
			);
		}

		for glyph in &run.glyphs {
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
		canvas.scene.fonts_seen.len(),
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

fn draw_dynamic_overlay(frame: &mut canvas::Frame, bounds: Rectangle, canvas: &GlyphCanvas, focused: bool) {
	let origin = scene_origin();

	if let Some(selection) = &canvas.editor.selection {
		if let Some(cluster_index) = canvas.scene.cluster_index_for_range(selection) {
			let cluster = &canvas.scene.clusters()[cluster_index];
			frame.fill_rectangle(
				Point::new(origin.x + cluster.x, origin.y + cluster.y),
				Size::new(cluster.width.max(1.0), cluster.height.max(1.0)),
				Color::from_rgba(1.0, 0.92, 0.45, 0.35),
			);
			frame.stroke_rectangle(
				Point::new(origin.x + cluster.x, origin.y + cluster.y),
				Size::new(cluster.width.max(1.0), cluster.height.max(1.0)),
				canvas::Stroke::default()
					.with_width(1.0)
					.with_color(Color::from_rgba(1.0, 0.95, 0.7, 0.92)),
			);
		}
	}

	if matches!(canvas.editor.mode, EditorMode::Insert) {
		let caret = canvas.scene.caret_metrics(canvas.editor.caret);
		let path = canvas::Path::line(
			Point::new(origin.x + caret.x, origin.y + caret.y),
			Point::new(origin.x + caret.x, origin.y + caret.y + caret.height),
		);
		frame.stroke(
			&path,
			canvas::Stroke::default()
				.with_width(2.0)
				.with_color(Color::from_rgba(0.98, 0.92, 0.7, 0.95)),
		);
	}

	if let Some(target) = canvas.hovered_target {
		draw_target_overlay(frame, canvas, origin, target, false);
	}

	if let Some(target) = canvas.selected_target {
		draw_target_overlay(frame, canvas, origin, target, true);
	}

	if focused {
		frame.stroke_rectangle(
			origin,
			Size::new(canvas.scene.max_width.max(1.0), canvas.scene.measured_height.max(1.0)),
			canvas::Stroke::default()
				.with_width(1.0)
				.with_color(Color::from_rgba(0.96, 0.92, 0.7, 0.85)),
		);
	}

	frame.fill_text(canvas::Text {
		content: format!("mode={} focus={focused}", canvas.editor.mode),
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
					canvas.scene.max_width.max(run.line_width).max(1.0),
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
			let Some(run) = canvas.scene.runs.get(run_index) else {
				return;
			};
			let Some(glyph) = run.glyphs.get(glyph_index) else {
				return;
			};

			let glyph_origin = Point::new(origin.x + glyph.x, origin.y + glyph.y);
			let glyph_size = Size::new(glyph.width.max(1.0), glyph.height.max(1.0));

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
					Size::new(glyph.width.max(0.5), glyph.height.max(0.5)),
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

fn key_command(
	mode: EditorMode, key: &keyboard::Key, physical_key: key::Physical, modifiers: keyboard::Modifiers,
	text: Option<&str>,
) -> Option<EditorCommand> {
	let latin = key
		.to_latin(physical_key)
		.map(|character| character.to_ascii_lowercase());

	match mode {
		EditorMode::Normal => {
			if modifiers.command() || modifiers.alt() {
				return None;
			}

			match key.as_ref() {
				key::Key::Named(key::Named::ArrowLeft) => Some(EditorCommand::MoveLeft),
				key::Key::Named(key::Named::ArrowRight) => Some(EditorCommand::MoveRight),
				key::Key::Named(key::Named::ArrowUp) => Some(EditorCommand::MoveUp),
				key::Key::Named(key::Named::ArrowDown) => Some(EditorCommand::MoveDown),
				key::Key::Named(key::Named::Home) => Some(EditorCommand::MoveLineStart),
				key::Key::Named(key::Named::End) => Some(EditorCommand::MoveLineEnd),
				key::Key::Named(key::Named::Backspace | key::Named::Delete) => Some(EditorCommand::DeleteSelection),
				key::Key::Named(key::Named::Enter) => Some(EditorCommand::EnterInsertAfter),
				key::Key::Named(key::Named::Escape) => Some(EditorCommand::ExitInsert),
				_ => match latin {
					Some('h') => Some(EditorCommand::MoveLeft),
					Some('l') => Some(EditorCommand::MoveRight),
					Some('k') => Some(EditorCommand::MoveUp),
					Some('j') => Some(EditorCommand::MoveDown),
					Some('i') => Some(EditorCommand::EnterInsertBefore),
					Some('a') => Some(EditorCommand::EnterInsertAfter),
					Some('x') => Some(EditorCommand::DeleteSelection),
					_ => None,
				},
			}
		}
		EditorMode::Insert => match key.as_ref() {
			key::Key::Named(key::Named::ArrowLeft) => Some(EditorCommand::MoveLeft),
			key::Key::Named(key::Named::ArrowRight) => Some(EditorCommand::MoveRight),
			key::Key::Named(key::Named::ArrowUp) => Some(EditorCommand::MoveUp),
			key::Key::Named(key::Named::ArrowDown) => Some(EditorCommand::MoveDown),
			key::Key::Named(key::Named::Home) => Some(EditorCommand::MoveLineStart),
			key::Key::Named(key::Named::End) => Some(EditorCommand::MoveLineEnd),
			key::Key::Named(key::Named::Backspace) => Some(EditorCommand::Backspace),
			key::Key::Named(key::Named::Delete) => Some(EditorCommand::DeleteForward),
			key::Key::Named(key::Named::Enter) => Some(EditorCommand::InsertText("\n".to_string())),
			key::Key::Named(key::Named::Tab) => Some(EditorCommand::InsertText("\t".to_string())),
			key::Key::Named(key::Named::Escape) => Some(EditorCommand::ExitInsert),
			_ => {
				if modifiers.command() || modifiers.alt() {
					return None;
				}

				text.filter(|text| !text.chars().all(char::is_control))
					.map(|text| EditorCommand::InsertText(text.to_string()))
			}
		},
	}
}

fn to_scene_local(position: Point) -> Point {
	Point::new(position.x - scene_origin().x, position.y - scene_origin().y)
}

fn scene_origin() -> Point {
	Point::new(24.0, 28.0)
}
