use {
	crate::{
		canvas_view::scene_origin,
		perf::CanvasPerfSink,
		presentation::ScenePresentation,
		types::{Message, WrapChoice},
	},
	iced::{
		Element, Length, Point, Rectangle, Size, Theme, Vector,
		advanced::{
			Layout, Renderer as _, Widget, graphics::geometry::Renderer as _, layout, mouse, renderer, widget::Tree,
		},
		widget::canvas,
	},
	std::{cell::Cell, time::Instant},
};

#[derive(Debug)]
struct StaticSceneState {
	cache: canvas::Cache<iced::Renderer>,
	cached_scene_key: Cell<Option<StaticSceneKey>>,
	cached_scene_size: Cell<Option<(u32, u32)>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StaticSceneKey {
	scene_revision: u64,
	show_baselines: bool,
	show_hitboxes: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct StaticSceneLayer {
	scene: ScenePresentation,
	layout_width: f32,
	show_baselines: bool,
	show_hitboxes: bool,
	scroll: Vector,
	perf: CanvasPerfSink,
	width: Length,
	height: Length,
}

impl StaticSceneLayer {
	pub(crate) fn new(
		scene: ScenePresentation, layout_width: f32, show_baselines: bool, show_hitboxes: bool, scroll: Vector,
		perf: CanvasPerfSink,
	) -> Self {
		Self {
			scene,
			layout_width,
			show_baselines,
			show_hitboxes,
			scroll,
			perf,
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
	fn cache_key(&self) -> StaticSceneKey {
		StaticSceneKey {
			scene_revision: self.scene.revision,
			show_baselines: self.show_baselines,
			show_hitboxes: self.show_hitboxes,
		}
	}
}

impl Widget<Message, Theme, iced::Renderer> for StaticSceneLayer {
	fn tag(&self) -> iced::advanced::widget::tree::Tag {
		iced::advanced::widget::tree::Tag::of::<StaticSceneState>()
	}

	fn state(&self) -> iced::advanced::widget::tree::State {
		iced::advanced::widget::tree::State::new(StaticSceneState {
			cache: canvas::Cache::default(),
			cached_scene_key: Cell::new(None),
			cached_scene_size: Cell::new(None),
		})
	}

	fn size(&self) -> Size<Length> {
		Size::new(self.width, self.height)
	}

	fn layout(&mut self, _tree: &mut Tree, _renderer: &iced::Renderer, limits: &layout::Limits) -> layout::Node {
		layout::Node::new(limits.resolve(self.width, self.height, Size::ZERO))
	}

	fn draw(
		&self, tree: &Tree, renderer: &mut iced::Renderer, _theme: &Theme, _style: &renderer::Style,
		layout: Layout<'_>, _cursor: mouse::Cursor, _viewport: &Rectangle,
	) {
		let started = Instant::now();
		let state = tree.state.downcast_ref::<StaticSceneState>();
		let bounds = layout.bounds();
		let scene_bounds = Rectangle::new(
			Point::ORIGIN,
			Size::new(
				scene_content_width(self.scene.layout.as_ref(), self.layout_width),
				self.scene.layout.measured_height.max(1.0),
			),
		);
		let scene_size_key = (
			scene_bounds.width.round().to_bits(),
			scene_bounds.height.round().to_bits(),
		);
		let cache_key = self.cache_key();
		let revision_changed = state.cached_scene_key.get() != Some(cache_key);
		let size_changed = state.cached_scene_size.get() != Some(scene_size_key);
		let cache_miss = revision_changed || size_changed;

		if revision_changed {
			state.cache.clear();
		}

		let mut static_build = None;
		let geometry = state.cache.draw_with_bounds(renderer, scene_bounds, |frame| {
			let build_started = Instant::now();
			draw_static_scene(frame, self);
			static_build = Some(build_started.elapsed());
		});

		if cache_miss {
			state.cached_scene_key.set(Some(cache_key));
			state.cached_scene_size.set(Some(scene_size_key));
		}

		renderer.with_layer(bounds, |renderer| {
			renderer.with_translation(
				Vector::new(
					bounds.x + scene_origin().x - self.scroll.x,
					bounds.y + scene_origin().y - self.scroll.y,
				),
				|renderer| renderer.draw_geometry(geometry),
			);
		});

		self.perf
			.record_canvas_draw(started.elapsed(), static_build, cache_miss);
	}
}

impl From<StaticSceneLayer> for Element<'_, Message> {
	fn from(widget: StaticSceneLayer) -> Self {
		Element::new(widget)
	}
}

fn draw_static_scene(frame: &mut canvas::Frame, layer: &StaticSceneLayer) {
	if layer.show_baselines {
		for run in layer.scene.layout.runs.iter() {
			let top_line = canvas::Path::line(
				Point::new(0.0, run.line_top),
				Point::new(layer.layout_width, run.line_top),
			);
			frame.stroke(
				&top_line,
				canvas::Stroke::default()
					.with_width(1.0)
					.with_color(iced::Color::from_rgba(1.0, 0.6, 0.2, 0.45)),
			);

			let baseline = canvas::Path::line(
				Point::new(0.0, run.baseline),
				Point::new(layer.layout_width, run.baseline),
			);
			frame.stroke(
				&baseline,
				canvas::Stroke::default()
					.with_width(1.0)
					.with_color(iced::Color::from_rgba(0.4, 1.0, 0.6, 0.45)),
			);
		}
	}

	if layer.show_hitboxes {
		for cluster in layer.scene.layout.clusters.iter() {
			frame.stroke_rectangle(
				Point::new(cluster.x, cluster.y),
				Size::new(cluster.width.max(0.5), cluster.height.max(0.5)),
				canvas::Stroke::default()
					.with_width(1.0)
					.with_color(iced::Color::from_rgba(1.0, 0.3, 0.3, 0.6)),
			);
		}
	}
}

fn scene_content_width(layout: &crate::scene::DocumentLayout, layout_width: f32) -> f32 {
	if matches!(layout.wrapping, WrapChoice::None) {
		layout.measured_width.max(layout_width).max(1.0)
	} else {
		layout_width.max(1.0)
	}
}
