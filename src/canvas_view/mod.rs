mod geometry;
mod input;
mod render;
mod state;

pub(crate) use geometry::{scene_origin, scene_viewport_size};
pub(crate) use state::CanvasState;

use std::sync::Arc;
use std::time::Instant;

use iced::widget::canvas;
use iced::{Rectangle, Theme, Vector, mouse};

use crate::editor::EditorViewState;
use crate::overlay::OverlayPrimitive;
use crate::perf::CanvasPerfSink;
use crate::scene::LayoutScene;
use crate::types::Message;

use self::geometry::max_scroll;
use self::input::decode_event;
use self::render::{draw_overlay, draw_static_scene, draw_underlay_overlay};

#[derive(Debug, Clone)]
pub(crate) struct GlyphCanvas {
	pub(crate) scene: LayoutScene,
	pub(crate) layout_width: f32,
	pub(crate) show_baselines: bool,
	pub(crate) show_hitboxes: bool,
	pub(crate) inspect_overlays: Arc<[OverlayPrimitive]>,
	pub(crate) editor: EditorViewState,
	pub(crate) scene_revision: u64,
	pub(crate) scroll: Vector,
	pub(crate) perf: CanvasPerfSink,
}

#[derive(Debug, Clone)]
pub(crate) struct GlyphCanvasUnderlay {
	pub(crate) editor: EditorViewState,
	pub(crate) scroll: Vector,
}

impl canvas::Program<Message> for GlyphCanvas {
	type State = CanvasState;

	fn update(
		&self, state: &mut Self::State, event: &canvas::Event, bounds: Rectangle, cursor: mouse::Cursor,
	) -> Option<canvas::Action<Message>> {
		let started = Instant::now();
		let max_scroll = max_scroll(bounds, &self.scene, self.layout_width);
		let action = decode_event(
			self.editor.mode,
			state.focused(),
			event,
			&self.scene,
			bounds,
			cursor,
			state.scroll(),
		)
		.map(|event| state.transition(event, max_scroll))
		.and_then(|action| action.into_iced());

		self.perf.record_canvas_update(started.elapsed());
		action
	}

	fn draw(
		&self, state: &Self::State, renderer: &iced::Renderer, _theme: &Theme, bounds: Rectangle,
		_cursor: iced::mouse::Cursor,
	) -> Vec<canvas::Geometry> {
		let started = Instant::now();
		let cache_miss = state.cache_miss(self.scene_revision, self.scroll);

		if cache_miss {
			state.refresh_cache_key(self.scene_revision, self.scroll);
		}

		let mut static_build = None;
		let static_layer = state.scene_cache().draw(renderer, bounds.size(), |frame| {
			let build_started = Instant::now();
			draw_static_scene(frame, bounds, self, self.scroll);
			static_build = Some(build_started.elapsed());
		});

		let overlay_started = Instant::now();
		let mut overlay = canvas::Frame::new(renderer, bounds.size());
		draw_overlay(&mut overlay, bounds, self, state.focused(), self.scroll);
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

impl canvas::Program<Message> for GlyphCanvasUnderlay {
	type State = ();

	fn draw(
		&self, _state: &Self::State, renderer: &iced::Renderer, _theme: &Theme, bounds: Rectangle,
		_cursor: iced::mouse::Cursor,
	) -> Vec<canvas::Geometry> {
		let mut overlay = canvas::Frame::new(renderer, bounds.size());
		draw_underlay_overlay(&mut overlay, &self.editor, self.scroll);
		vec![overlay.into_geometry()]
	}
}
