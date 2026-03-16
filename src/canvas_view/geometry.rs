use {
	crate::scene::LayoutScene,
	iced::{Point, Rectangle, Size, Vector, mouse},
	std::time::Duration,
};

pub(super) const DOUBLE_CLICK_INTERVAL: Duration = Duration::from_millis(300);
pub(super) const DOUBLE_CLICK_DISTANCE: f32 = 8.0;

pub(crate) fn scene_origin() -> Point {
	Point::new(24.0, 28.0)
}

pub(crate) fn scene_viewport_size(bounds: Size) -> Size {
	Size::new(
		(bounds.width - scene_origin().x - 24.0).max(1.0),
		(bounds.height - scene_origin().y - 36.0).max(1.0),
	)
}

pub(super) fn viewport_size(bounds: Rectangle) -> Size {
	scene_viewport_size(bounds.size())
}

pub(super) fn clamp_scroll(scroll: Vector, max_scroll: Vector) -> Vector {
	Vector::new(scroll.x.clamp(0.0, max_scroll.x), scroll.y.clamp(0.0, max_scroll.y))
}

pub(super) fn animate_scroll(current: Vector, target: Vector) -> Vector {
	current + ((target - current) * 0.22)
}

pub(super) fn max_scroll(bounds: Rectangle, scene: &LayoutScene, layout_width: f32) -> Vector {
	let viewport = viewport_size(bounds);
	Vector::new(
		(scene_content_width(scene, layout_width) - viewport.width).max(0.0),
		(scene.measured_height - viewport.height).max(0.0),
	)
}

pub(super) fn scroll_delta(delta: mouse::ScrollDelta) -> Vector {
	match delta {
		mouse::ScrollDelta::Lines { x, y } => -Vector::new(x, y) * 60.0,
		mouse::ScrollDelta::Pixels { x, y } => -Vector::new(x, y),
	}
}

pub(super) fn vector_length(vector: Vector) -> f32 {
	(vector.x * vector.x + vector.y * vector.y).sqrt()
}

pub(super) fn point_distance(a: Point, b: Point) -> f32 {
	let dx = a.x - b.x;
	let dy = a.y - b.y;
	(dx * dx + dy * dy).sqrt()
}

pub(super) fn to_scene_local(position: Point, scroll: Vector) -> Point {
	Point::new(
		position.x - scene_origin().x + scroll.x,
		position.y - scene_origin().y + scroll.y,
	)
}

fn scene_content_width(scene: &LayoutScene, layout_width: f32) -> f32 {
	if matches!(scene.wrapping, crate::types::WrapChoice::None) {
		scene.measured_width.max(layout_width)
	} else {
		layout_width
	}
}

#[cfg(test)]
mod tests {
	use {
		super::{clamp_scroll, max_scroll},
		crate::scene::{LayoutScene, LayoutSceneTestSpec},
		iced::{Rectangle, Vector},
		std::sync::Arc,
	};

	fn scene(width: f32, height: f32) -> LayoutScene {
		LayoutScene::new_for_test(LayoutSceneTestSpec {
			text: Arc::<str>::from(""),
			wrapping: crate::types::WrapChoice::Word,
			render_mode: crate::types::RenderMode::CanvasOnly,
			font_size: 16.0,
			line_height: 20.0,
			max_width: width,
			measured_width: width,
			measured_height: height,
			glyph_count: 0,
			font_count: 0,
			runs: Vec::new(),
			clusters: Vec::new(),
		})
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

		let max = max_scroll(bounds, &scene, scene.max_width);
		assert!(max.x > 0.0);
		assert!(max.y > 0.0);
		assert_eq!(clamp_scroll(Vector::new(-10.0, 2000.0), max), Vector::new(0.0, max.y));
	}
}
