use iced::advanced::Widget;
use iced::advanced::layout;
use iced::advanced::renderer;
use iced::advanced::text::Paragraph as _;
use iced::advanced::text::Renderer as _;
use iced::advanced::widget::Tree;
use iced::advanced::{Layout, Renderer as _, mouse};
use iced::{Color, Element, Length, Point, Rectangle, Size, Theme, Vector};

use crate::canvas_view::scene_origin;
use crate::editor::{EditorMode, EditorViewState};
use crate::scene::LayoutScene;
use crate::types::Message;

const OUTER_BACKGROUND: Color = Color::from_rgb8(20, 24, 32);
const TEXT_BACKGROUND: Color = Color::from_rgb8(28, 34, 46);
const TEXT_COLOR: Color = Color::from_rgba(0.4, 0.8, 1.0, 0.9);
const INSERT_GLYPH_COLOR: Color = Color::from_rgb8(8, 14, 24);
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
	editor: EditorViewState,
	layout_width: f32,
	scroll: Vector,
	draw_backdrop: bool,
	draw_text: bool,
	width: Length,
	height: Length,
}

impl SceneTextLayer {
	pub(crate) fn new(scene: LayoutScene, editor: EditorViewState, layout_width: f32, scroll: Vector) -> Self {
		Self {
			scene,
			editor,
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
		}

		if self.draw_text && self.scene.draw_canvas_text {
			renderer.fill_paragraph(&state.paragraph, origin, TEXT_COLOR, bounds);
			if let Some(clip) = insert_repaint_clip(origin, self.editor.mode, self.editor.viewport_target) {
				renderer.fill_paragraph(&state.paragraph, origin, INSERT_GLYPH_COLOR, clip);
			}
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

fn insert_repaint_clip(
	origin: Point, mode: EditorMode, target: Option<crate::overlay::LayoutRect>,
) -> Option<Rectangle> {
	matches!(mode, EditorMode::Insert).then_some(())?;
	let target = target?;
	Some(Rectangle::new(
		Point::new(origin.x + target.x, origin.y + target.y),
		Size::new(target.width.max(1.0), target.height.max(1.0)),
	))
}

#[cfg(test)]
mod tests {
	use super::insert_repaint_clip;
	use crate::editor::EditorMode;
	use crate::overlay::LayoutRect;
	use iced::{Point, Rectangle, Size};

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
