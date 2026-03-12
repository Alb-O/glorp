use iced::advanced::Widget;
use iced::advanced::layout;
use iced::advanced::renderer;
use iced::advanced::text::Paragraph as _;
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

#[derive(Debug)]
struct ParagraphState {
	paragraph: iced::advanced::graphics::text::Paragraph,
	text: std::sync::Arc<str>,
}

#[derive(Debug, Clone)]
pub(crate) struct SceneTextLayer {
	scene: LayoutScene,
	layout_width: f32,
	scroll: Vector,
	width: Length,
	height: Length,
}

impl SceneTextLayer {
	pub(crate) fn new(scene: LayoutScene, layout_width: f32, scroll: Vector) -> Self {
		Self {
			scene,
			layout_width,
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
	fn tag(&self) -> iced::advanced::widget::tree::Tag {
		iced::advanced::widget::tree::Tag::of::<ParagraphState>()
	}

	fn state(&self) -> iced::advanced::widget::tree::State {
		iced::advanced::widget::tree::State::new(ParagraphState {
			paragraph: iced::advanced::graphics::text::Paragraph::with_text(text_spec(&self.scene, self.layout_width)),
			text: self.scene.text.clone(),
		})
	}

	fn size(&self) -> Size<Length> {
		Size::new(self.width, self.height)
	}

	fn layout(&mut self, tree: &mut Tree, _renderer: &iced::Renderer, limits: &layout::Limits) -> layout::Node {
		let state = tree.state.downcast_mut::<ParagraphState>();
		sync_paragraph_state(&self.scene, self.layout_width, state);

		layout::Node::new(limits.resolve(
			self.width,
			self.height,
			Size::new(self.layout_width.max(1.0), self.scene.measured_height.max(1.0)),
		))
	}

	fn draw(
		&self, tree: &Tree, renderer: &mut iced::Renderer, _theme: &Theme, _style: &renderer::Style,
		layout: Layout<'_>, _cursor: mouse::Cursor, _viewport: &Rectangle,
	) {
		let state = tree.state.downcast_ref::<ParagraphState>();
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
						self.layout_width.max(1.0),
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
					Size::new(self.layout_width.max(1.0), self.scene.measured_height.max(1.0)),
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
			renderer.fill_paragraph(&state.paragraph, origin, TEXT_COLOR, bounds);
		}
	}
}

impl<'a> From<SceneTextLayer> for Element<'a, Message> {
	fn from(widget: SceneTextLayer) -> Self {
		Element::new(widget)
	}
}

fn sync_paragraph_state(scene: &LayoutScene, layout_width: f32, state: &mut ParagraphState) {
	if state.text != scene.text {
		state.paragraph = iced::advanced::graphics::text::Paragraph::with_text(text_spec(scene, layout_width));
		state.text = scene.text.clone();
		return;
	}

	sync_paragraph_with_width(scene, layout_width, &mut state.paragraph);
}

fn sync_paragraph_with_width(
	scene: &LayoutScene, layout_width: f32, paragraph: &mut iced::advanced::graphics::text::Paragraph,
) {
	let text = text_spec(scene, layout_width);
	match paragraph.compare(iced::advanced::text::Text {
		content: (),
		bounds: text.bounds,
		size: text.size,
		line_height: text.line_height,
		font: text.font,
		align_x: text.align_x,
		align_y: text.align_y,
		shaping: text.shaping,
		wrapping: text.wrapping,
	}) {
		iced::advanced::text::Difference::None => {}
		iced::advanced::text::Difference::Bounds => paragraph.resize(text.bounds),
		iced::advanced::text::Difference::Shape => {
			*paragraph = iced::advanced::graphics::text::Paragraph::with_text(text);
		}
	}
}

fn text_spec(scene: &LayoutScene, layout_width: f32) -> iced::advanced::text::Text<&str> {
	iced::advanced::text::Text {
		content: &scene.text,
		bounds: Size::new(
			if matches!(scene.wrapping, crate::types::WrapChoice::None) {
				f32::INFINITY
			} else {
				layout_width
			},
			f32::INFINITY,
		),
		size: iced::Pixels(scene.font_size),
		line_height: iced::advanced::text::LineHeight::Absolute(iced::Pixels(scene.line_height)),
		font: scene.font_choice.to_iced_font(),
		align_x: iced::advanced::text::Alignment::Left,
		align_y: iced::alignment::Vertical::Top,
		shaping: scene.shaping.to_iced(),
		wrapping: scene.wrapping.to_iced(),
	}
}
