use iced::advanced::Widget;
use iced::advanced::layout;
use iced::advanced::renderer;
use iced::advanced::text::Renderer as _;
use iced::advanced::widget::Tree;
use iced::advanced::{Layout, Renderer as _, mouse};
use iced::{Color, Element, Length, Point, Rectangle, Size, Theme, Vector};

use crate::canvas_view::scene_origin;
use crate::scene::LayoutScene;
use crate::types::Message;

const OUTER_BACKGROUND: Color = Color::from_rgb8(20, 24, 32);
const TEXT_BACKGROUND: Color = Color::from_rgb8(28, 34, 46);
const TEXT_COLOR: Color = Color::from_rgba(0.4, 0.8, 1.0, 0.9);
const GUIDE_COLOR: Color = Color::from_rgba(0.6, 0.7, 1.0, 0.18);
const BORDER_COLOR: Color = Color::from_rgba(0.8, 0.8, 0.9, 0.65);

#[derive(Debug, Clone)]
pub(crate) struct SceneTextLayer {
	scene: LayoutScene,
	scroll: Vector,
	width: Length,
	height: Length,
}

impl SceneTextLayer {
	pub(crate) fn new(scene: LayoutScene, scroll: Vector) -> Self {
		Self {
			scene,
			scroll,
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

impl Widget<Message, Theme, iced::Renderer> for SceneTextLayer {
	fn size(&self) -> Size<Length> {
		Size::new(self.width, self.height)
	}

	fn layout(&mut self, _tree: &mut Tree, _renderer: &iced::Renderer, limits: &layout::Limits) -> layout::Node {
		layout::Node::new(limits.resolve(
			self.width,
			self.height,
			Size::new(self.scene.max_width.max(1.0), self.scene.measured_height.max(1.0)),
		))
	}

	fn draw(
		&self, _tree: &Tree, renderer: &mut iced::Renderer, _theme: &Theme, _style: &renderer::Style,
		layout: Layout<'_>, _cursor: mouse::Cursor, _viewport: &Rectangle,
	) {
		let bounds = layout.bounds();
		let origin = Point::new(
			bounds.x + scene_origin().x - self.scroll.x,
			bounds.y + scene_origin().y - self.scroll.y,
		);

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
						self.scene.max_width.max(1.0),
						self.scene
							.measured_height
							.max(bounds.height - origin.y + bounds.y - 24.0),
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
				bounds: Rectangle::new(
					origin,
					Size::new(self.scene.max_width.max(1.0), self.scene.measured_height.max(1.0)),
				),
				border: iced::Border {
					width: 1.0,
					color: BORDER_COLOR,
					..iced::Border::default()
				},
				..renderer::Quad::default()
			},
			Color::TRANSPARENT,
		);

		if self.scene.draw_canvas_text {
			renderer.fill_paragraph(&self.scene.paragraph, origin, TEXT_COLOR, bounds);
		}
	}
}

impl<'a> From<SceneTextLayer> for Element<'a, Message> {
	fn from(widget: SceneTextLayer) -> Self {
		Element::new(widget)
	}
}
