use std::cell::Cell;
use std::time::Instant;

use iced::keyboard::{self, key};
use iced::widget::canvas;
use iced::{Color, Font, Pixels, Point, Rectangle, Size, Theme, Vector, mouse, window};

use crate::editor::{EditorCommand, EditorMode, EditorViewState};
use crate::perf::PerfBridge;
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
	pub(crate) scroll: Vector,
	pub(crate) perf: PerfBridge,
}

#[derive(Debug, Default)]
pub(crate) struct CanvasState {
	hovered_target: Option<CanvasTarget>,
	focused: bool,
	scroll: Vector,
	target_scroll: Vector,
	scene_cache: canvas::Cache,
	cached_scene_revision: Cell<Option<u64>>,
	cached_scroll: Cell<Option<(i32, i32)>>,
}

impl canvas::Program<Message> for GlyphCanvas {
	type State = CanvasState;

	fn update(
		&self, state: &mut Self::State, event: &canvas::Event, bounds: Rectangle, cursor: mouse::Cursor,
	) -> Option<canvas::Action<Message>> {
		let started = Instant::now();
		let previous_scroll = state.scroll;
		let max_scroll = max_scroll(bounds, &self.scene);
		state.target_scroll = clamp_scroll(state.target_scroll, max_scroll);
		state.scroll = clamp_scroll(state.scroll, max_scroll);

		let cursor_position = cursor.position_in(bounds);
		let cursor_local = cursor_position.map(|position| to_scene_local(position, state.scroll));
		let cursor_target = cursor_local.and_then(|position| self.scene.hit_test(position));

		let action = match event {
			canvas::Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
				if !cursor.is_over(bounds) {
					None
				} else {
					state.focused = true;
					state.target_scroll = clamp_scroll(state.target_scroll + scroll_delta(*delta), max_scroll);

					if vector_length(state.target_scroll - state.scroll) > 0.1 {
						Some(canvas::Action::request_redraw().and_capture())
					} else {
						None
					}
				}
			}
			canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
				if state.hovered_target != cursor_target {
					state.hovered_target = cursor_target;
					Some(canvas::Action::publish(Message::CanvasHovered(cursor_target)))
				} else {
					None
				}
			}
			canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
				if !cursor.is_over(bounds) {
					state.focused = false;
					None
				} else if let Some(position) = cursor_local {
					state.focused = true;
					state.hovered_target = cursor_target;

					Some(
						canvas::Action::publish(Message::CanvasClicked {
							target: cursor_target,
							position,
						})
						.and_capture(),
					)
				} else {
					None
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
					Some(canvas::Action::publish(Message::EditorCommand(command)).and_capture())
				} else {
					None
				}
			}
			canvas::Event::Window(window::Event::RedrawRequested(_now)) => {
				let next_scroll = animate_scroll(state.scroll, state.target_scroll);
				if vector_length(next_scroll - state.scroll) > 0.01 {
					state.scroll = clamp_scroll(next_scroll, max_scroll);
					Some(canvas::Action::publish(Message::CanvasScrollChanged(state.scroll)))
				} else {
					state.scroll = clamp_scroll(state.target_scroll, max_scroll);
					(vector_length(state.scroll - previous_scroll) > 0.01)
						.then_some(canvas::Action::publish(Message::CanvasScrollChanged(state.scroll)))
				}
			}
			_ => {
				if !cursor.is_over(bounds) && state.hovered_target.is_some() {
					state.hovered_target = None;
					Some(canvas::Action::publish(Message::CanvasHovered(None)))
				} else {
					None
				}
			}
		};

		self.perf.record_canvas_update(started.elapsed());
		action
	}

	fn draw(
		&self, state: &Self::State, renderer: &iced::Renderer, _theme: &Theme, bounds: Rectangle,
		_cursor: iced::mouse::Cursor,
	) -> Vec<canvas::Geometry> {
		let started = Instant::now();
		let cached_scroll = (self.scroll.x.round() as i32, self.scroll.y.round() as i32);
		let cache_miss = state.cached_scene_revision.get() != Some(self.scene_revision)
			|| state.cached_scroll.get() != Some(cached_scroll);

		if cache_miss {
			state.scene_cache.clear();
			state.cached_scene_revision.set(Some(self.scene_revision));
			state.cached_scroll.set(Some(cached_scroll));
		}

		let mut static_build = None;
		let static_layer = state.scene_cache.draw(renderer, bounds.size(), |frame| {
			let build_started = Instant::now();
			draw_static_scene(frame, bounds, self, self.scroll);
			static_build = Some(build_started.elapsed());
		});

		let overlay_started = Instant::now();
		let mut overlay = canvas::Frame::new(renderer, bounds.size());
		draw_dynamic_overlay(&mut overlay, bounds, self, state.focused, self.scroll);
		let overlay_elapsed = overlay_started.elapsed();

		let geometry = vec![static_layer, overlay.into_geometry()];
		self.perf
			.record_canvas_draw(started.elapsed(), static_build, overlay_elapsed, cache_miss);
		geometry
	}

	fn mouse_interaction(&self, _state: &Self::State, bounds: Rectangle, cursor: mouse::Cursor) -> mouse::Interaction {
		cursor
			.position_in(bounds)
			.map(|_| mouse::Interaction::Text)
			.unwrap_or_default()
	}
}

fn draw_static_scene(frame: &mut canvas::Frame, bounds: Rectangle, canvas: &GlyphCanvas, scroll: Vector) {
	let origin = scrolled_origin(scroll);
	let visible_scene_bounds = visible_scene_bounds(bounds, scroll);
	for run in canvas.scene.runs.iter() {
		if !run_intersects_viewport(run, visible_scene_bounds) {
			continue;
		}

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

fn draw_dynamic_overlay(
	frame: &mut canvas::Frame, bounds: Rectangle, canvas: &GlyphCanvas, focused: bool, scroll: Vector,
) {
	let origin = scrolled_origin(scroll);

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

fn clamp_scroll(scroll: Vector, max_scroll: Vector) -> Vector {
	Vector::new(scroll.x.clamp(0.0, max_scroll.x), scroll.y.clamp(0.0, max_scroll.y))
}

fn animate_scroll(current: Vector, target: Vector) -> Vector {
	current + ((target - current) * 0.22)
}

fn max_scroll(bounds: Rectangle, scene: &LayoutScene) -> Vector {
	let viewport = viewport_size(bounds);
	Vector::new(
		(scene.max_width - viewport.width).max(0.0),
		(scene.measured_height - viewport.height).max(0.0),
	)
}

fn scroll_delta(delta: mouse::ScrollDelta) -> Vector {
	match delta {
		mouse::ScrollDelta::Lines { x, y } => -Vector::new(x, y) * 60.0,
		mouse::ScrollDelta::Pixels { x, y } => -Vector::new(x, y),
	}
}

fn vector_length(vector: Vector) -> f32 {
	(vector.x * vector.x + vector.y * vector.y).sqrt()
}

fn to_scene_local(position: Point, scroll: Vector) -> Point {
	Point::new(
		position.x - scene_origin().x + scroll.x,
		position.y - scene_origin().y + scroll.y,
	)
}

fn scrolled_origin(scroll: Vector) -> Point {
	Point::new(scene_origin().x - scroll.x, scene_origin().y - scroll.y)
}

fn viewport_size(bounds: Rectangle) -> Size {
	Size::new(
		(bounds.width - scene_origin().x - 24.0).max(1.0),
		(bounds.height - scene_origin().y - 36.0).max(1.0),
	)
}

pub(crate) fn scene_origin() -> Point {
	Point::new(24.0, 28.0)
}

fn visible_scene_bounds(bounds: Rectangle, scroll: Vector) -> Rectangle {
	Rectangle::new(Point::new(scroll.x, scroll.y), viewport_size(bounds))
}

fn run_intersects_viewport(run: &crate::scene::RunInfo, viewport: Rectangle) -> bool {
	let run_bottom = run.line_top + run.line_height;
	run_bottom >= viewport.y && run.line_top <= viewport.y + viewport.height
}

fn glyph_intersects_viewport(glyph: &crate::scene::GlyphInfo, viewport: Rectangle) -> bool {
	let glyph_right = glyph.x + glyph.width.max(1.0);
	let glyph_bottom = glyph.y + glyph.height.max(1.0);

	glyph_right >= viewport.x
		&& glyph.x <= viewport.x + viewport.width
		&& glyph_bottom >= viewport.y
		&& glyph.y <= viewport.y + viewport.height
}

#[cfg(test)]
mod tests {
	use super::{clamp_scroll, max_scroll};
	use crate::scene::LayoutScene;
	use iced::{Rectangle, Vector};
	use std::sync::Arc;

	fn scene(width: f32, height: f32) -> LayoutScene {
		LayoutScene {
			text: Arc::<str>::from(""),
			font_choice: crate::types::FontChoice::Monospace,
			shaping: crate::types::ShapingChoice::Basic,
			wrapping: crate::types::WrapChoice::Word,
			render_mode: crate::types::RenderMode::CanvasOnly,
			font_size: 16.0,
			line_height: 20.0,
			max_width: width,
			measured_width: width,
			measured_height: height,
			glyph_count: 0,
			font_count: 0,
			runs: Vec::new().into(),
			clusters: Vec::new().into(),
			warnings: Vec::new().into(),
			draw_canvas_text: true,
			draw_outlines: false,
		}
	}

	#[test]
	fn canvas_scroll_is_clamped_to_scene_extent() {
		let scene = scene(1200.0, 1600.0);
		let bounds = Rectangle {
			x: 0.0,
			y: 0.0,
			width: 900.0,
			height: 700.0,
		};

		let max = max_scroll(bounds, &scene);
		assert!(max.x > 0.0);
		assert!(max.y > 0.0);
		assert_eq!(clamp_scroll(Vector::new(-10.0, 2000.0), max), Vector::new(0.0, max.y));
	}
}
