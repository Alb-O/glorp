use {
	crate::{canvas_view::scene_origin, editor::EditorMode, presentation::SessionSnapshot, types::Message},
	iced::{
		Color, Element, Length, Point, Rectangle, Size, Theme, Vector,
		advanced::{Layout, Renderer as _, Widget, graphics::text::Renderer as _, layout, mouse, renderer},
	},
};

const OUTER_BACKGROUND: Color = Color::from_rgb8(20, 24, 32);
const TEXT_BACKGROUND: Color = Color::from_rgb8(28, 34, 46);
const TEXT_COLOR: Color = Color::from_rgba(0.4, 0.8, 1.0, 0.9);
const INSERT_GLYPH_COLOR: Color = Color::from_rgb8(8, 14, 24);
const GUIDE_COLOR: Color = Color::from_rgba(0.6, 0.7, 1.0, 0.18);
const BORDER_COLOR: Color = Color::from_rgba(0.8, 0.8, 0.9, 0.65);

#[derive(Debug, Clone)]
pub(crate) struct SceneTextLayer {
	snapshot: SessionSnapshot,
	layout_width: f32,
	scroll: Vector,
	draw_backdrop: bool,
	draw_text: bool,
	width: Length,
	height: Length,
}

impl SceneTextLayer {
	pub(crate) fn new(snapshot: SessionSnapshot, layout_width: f32, scroll: Vector) -> Self {
		Self {
			snapshot,
			layout_width,
			scroll,
			draw_backdrop: true,
			draw_text: true,
			width: Length::Fill,
			height: Length::Fill,
		}
	}

	pub(crate) fn backdrop_only(mut self) -> Self {
		self.draw_backdrop = true;
		self.draw_text = false;
		self
	}

	pub(crate) fn text_only(mut self) -> Self {
		self.draw_backdrop = false;
		self.draw_text = true;
		self
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

impl Widget<Message, Theme, iced::Renderer> for SceneTextLayer {
	fn size(&self) -> Size<Length> {
		Size::new(self.width, self.height)
	}

	fn layout(
		&mut self, _tree: &mut iced::advanced::widget::Tree, _renderer: &iced::Renderer, limits: &layout::Limits,
	) -> layout::Node {
		layout::Node::new(limits.resolve(
			self.width,
			self.height,
			Size::new(
				self.layout_width.max(1.0),
				self.snapshot.editor.text_layer.measured_height.max(1.0),
			),
		))
	}

	fn draw(
		&self, _tree: &iced::advanced::widget::Tree, renderer: &mut iced::Renderer, _theme: &Theme,
		_style: &renderer::Style, layout: Layout<'_>, _cursor: mouse::Cursor, _viewport: &Rectangle,
	) {
		let bounds = layout.bounds();
		let content_width = self.layout_width.max(1.0);
		let text_height = self.snapshot.editor.text_layer.measured_height.max(1.0);
		let origin = Point::new(
			bounds.x + scene_origin().x - self.scroll.x,
			bounds.y + scene_origin().y - self.scroll.y,
		);

		if self.draw_backdrop {
			renderer.fill_quad(
				renderer::Quad {
					bounds,
					..renderer::Quad::default()
				},
				OUTER_BACKGROUND,
			);

			renderer.fill_quad(
				renderer::Quad {
					bounds: Rectangle::new(
						origin,
						Size::new(
							content_width,
							text_height.max(bounds.height - origin.y + bounds.y - 24.0),
						),
					),
					..renderer::Quad::default()
				},
				TEXT_BACKGROUND,
			);

			renderer.fill_quad(
				renderer::Quad {
					bounds: Rectangle::new(Point::new(origin.x, bounds.y), Size::new(1.0, bounds.height)),
					..renderer::Quad::default()
				},
				GUIDE_COLOR,
			);

			renderer.fill_quad(
				renderer::Quad {
					bounds: Rectangle::new(origin, Size::new(content_width, text_height)),
					border: iced::Border {
						width: 1.0,
						color: BORDER_COLOR,
						..iced::Border::default()
					},
					..renderer::Quad::default()
				},
				Color::TRANSPARENT,
			);
		}

		if self.draw_text {
			if let Some(clip) = insert_repaint_clip(
				origin,
				self.snapshot.editor.editor.mode,
				self.snapshot.editor.editor.viewport_target,
			) {
				let buffer = self.snapshot.editor.text_layer.buffer.clone();
				// Draw once in the normal color, then repaint only the active insert
				// cell with the inverted glyph color.
				renderer.fill_raw(iced::advanced::graphics::text::Raw {
					buffer: buffer.clone(),
					position: origin,
					color: TEXT_COLOR,
					clip_bounds: bounds,
				});
				renderer.fill_raw(iced::advanced::graphics::text::Raw {
					buffer,
					position: origin,
					color: INSERT_GLYPH_COLOR,
					clip_bounds: clip,
				});
			} else {
				renderer.fill_raw(iced::advanced::graphics::text::Raw {
					buffer: self.snapshot.editor.text_layer.buffer.clone(),
					position: origin,
					color: TEXT_COLOR,
					clip_bounds: bounds,
				});
			}
		}
	}
}

impl From<SceneTextLayer> for Element<'_, Message> {
	fn from(widget: SceneTextLayer) -> Self {
		Element::new(widget)
	}
}

fn insert_repaint_clip(
	origin: Point, mode: EditorMode, target: Option<crate::overlay::LayoutRect>,
) -> Option<Rectangle> {
	let target = target.filter(|_| matches!(mode, EditorMode::Insert))?;
	Some(Rectangle::new(
		Point::new(origin.x + target.x, origin.y + target.y),
		Size::new(target.width.max(1.0), target.height.max(1.0)),
	))
}

#[cfg(test)]
mod tests {
	use {
		super::insert_repaint_clip,
		crate::{editor::EditorMode, overlay::LayoutRect},
		iced::{Point, Rectangle, Size},
	};

	#[test]
	fn insert_repaint_clip_requires_insert_mode() {
		let clip = insert_repaint_clip(
			Point::new(10.0, 20.0),
			EditorMode::Normal,
			Some(LayoutRect {
				x: 2.0,
				y: 4.0,
				width: 8.0,
				height: 12.0,
			}),
		);

		assert_eq!(clip, None);
	}

	#[test]
	fn insert_repaint_clip_requires_a_target_rect() {
		let clip = insert_repaint_clip(Point::new(10.0, 20.0), EditorMode::Insert, None);

		assert_eq!(clip, None);
	}

	#[test]
	fn insert_repaint_clip_offsets_from_scene_origin_and_clamps_size() {
		let clip = insert_repaint_clip(
			Point::new(10.0, 20.0),
			EditorMode::Insert,
			Some(LayoutRect {
				x: 2.5,
				y: 4.5,
				width: 0.0,
				height: -3.0,
			}),
		);

		assert_eq!(clip, Some(Rectangle::new(Point::new(12.5, 24.5), Size::new(1.0, 1.0))));
	}
}
