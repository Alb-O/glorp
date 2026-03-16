mod geometry;
mod input;
mod state;

use {
	self::{geometry::max_scroll, input::decode_event},
	crate::{perf::CanvasPerfSink, presentation::DocumentPresentation, types::Message},
	iced::{Rectangle, Theme, mouse, widget::canvas},
	std::time::Instant,
	tracing::trace_span,
};
pub(crate) use {
	geometry::{scene_origin, scene_viewport_size},
	state::CanvasState,
};

#[derive(Debug, Clone)]
pub(crate) struct GlyphCanvas {
	pub(crate) presentation: DocumentPresentation,
	pub(crate) layout_width: f32,
	pub(crate) perf: CanvasPerfSink,
}

impl canvas::Program<Message> for GlyphCanvas {
	type State = CanvasState;

	fn update(
		&self, state: &mut Self::State, event: &canvas::Event, bounds: Rectangle, cursor: mouse::Cursor,
	) -> Option<canvas::Action<Message>> {
		let _span = trace_span!(
			"canvas.update",
			bounds_width = bounds.width,
			bounds_height = bounds.height
		)
		.entered();
		let started = Instant::now();
		let max_scroll = max_scroll(bounds, self.presentation.layout.as_ref(), self.layout_width);
		let action = decode_event(
			self.presentation.mode(),
			state.focused(),
			event,
			self.presentation.layout.as_ref(),
			bounds,
			cursor,
			state.scroll(),
		)
		.map(|event| state.transition(event, max_scroll))
		.and_then(state::CanvasAction::into_iced);

		self.perf.record_canvas_update(started.elapsed());
		action
	}

	fn draw(
		&self, _state: &Self::State, _renderer: &iced::Renderer, _theme: &Theme, _bounds: Rectangle,
		_cursor: iced::mouse::Cursor,
	) -> Vec<canvas::Geometry> {
		Vec::new()
	}

	fn mouse_interaction(&self, _state: &Self::State, bounds: Rectangle, cursor: mouse::Cursor) -> mouse::Interaction {
		cursor
			.position_in(bounds)
			.map(|_| mouse::Interaction::Text)
			.unwrap_or_default()
	}
}
