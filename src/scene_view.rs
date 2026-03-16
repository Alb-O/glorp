use {
	crate::{
		canvas_view::scene_origin,
		perf::CanvasPerfSink,
		presentation::DocumentPresentation,
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
	cached_scene_revision: Cell<Option<u64>>,
	cached_scene_size: Cell<Option<(u32, u32)>>,
	cached_scene_has_debug_geometry: Cell<bool>,
}

#[derive(Debug, Clone)]
pub(crate) struct StaticSceneLayer {
	presentation: DocumentPresentation,
	layout_width: f32,
	show_baselines: bool,
	show_hitboxes: bool,
	scene_revision: u64,
	scroll: Vector,
	perf: CanvasPerfSink,
	width: Length,
	height: Length,
}

impl StaticSceneLayer {
	pub(crate) fn new(
		presentation: DocumentPresentation, layout_width: f32, show_baselines: bool, show_hitboxes: bool,
		scene_revision: u64, scroll: Vector, perf: CanvasPerfSink,
	) -> Self {
		Self {
			presentation,
			layout_width,
			show_baselines,
			show_hitboxes,
			scene_revision,
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

	fn has_debug_geometry(&self) -> bool {
		self.show_baselines || self.show_hitboxes
	}
}

impl Widget<Message, Theme, iced::Renderer> for StaticSceneLayer {
	fn tag(&self) -> iced::advanced::widget::tree::Tag {
		iced::advanced::widget::tree::Tag::of::<StaticSceneState>()
	}

	fn state(&self) -> iced::advanced::widget::tree::State {
		iced::advanced::widget::tree::State::new(StaticSceneState {
			cache: canvas::Cache::default(),
			cached_scene_revision: Cell::new(None),
			cached_scene_size: Cell::new(None),
			cached_scene_has_debug_geometry: Cell::new(false),
		})
	}

	fn diff(&self, tree: &mut Tree) {
		let state = tree.state.downcast_mut::<StaticSceneState>();

		// Replacing the cache object is only needed when a prior debug-heavy
		// build would otherwise donate its larger mesh storage to a plain
		// document scene again.
		if should_reset_cache_storage(
			state.cached_scene_revision.get(),
			state.cached_scene_has_debug_geometry.get(),
			self.scene_revision,
			self.has_debug_geometry(),
		) {
			state.cache = canvas::Cache::default();
			state.cached_scene_revision.set(None);
			state.cached_scene_size.set(None);
			state.cached_scene_has_debug_geometry.set(false);
		}
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
				scene_content_width(self.presentation.layout.as_ref(), self.layout_width),
				self.presentation.layout.measured_height.max(1.0),
			),
		);
		let scene_size_key = (
			scene_bounds.width.round().to_bits(),
			scene_bounds.height.round().to_bits(),
		);
		// `scene_revision` deliberately includes decoration-only invalidations
		// such as baseline/hitbox toggles that do not change the presentation.
		let revision_changed = state.cached_scene_revision.get() != Some(self.scene_revision);
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
			state.cached_scene_revision.set(Some(self.scene_revision));
			state.cached_scene_size.set(Some(scene_size_key));
			state.cached_scene_has_debug_geometry.set(self.has_debug_geometry());
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
		for run in layer.presentation.layout.runs.iter() {
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
		for cluster in layer.presentation.layout.clusters.iter() {
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

fn should_reset_cache_storage(
	cached_scene_revision: Option<u64>, cached_scene_has_debug_geometry: bool, scene_revision: u64,
	scene_has_debug_geometry: bool,
) -> bool {
	cached_scene_revision != Some(scene_revision) && cached_scene_has_debug_geometry && !scene_has_debug_geometry
}

#[cfg(test)]
mod tests {
	use super::should_reset_cache_storage;

	#[test]
	fn drops_cached_storage_when_debug_geometry_is_removed() {
		assert!(should_reset_cache_storage(Some(4), true, 5, false));
	}

	#[test]
	fn keeps_cached_storage_while_debug_geometry_remains() {
		assert!(!should_reset_cache_storage(Some(4), true, 5, true));
	}
}
