mod geometry;
mod input;
mod state;

use {
	self::{geometry::max_scroll, input::decode_event},
	crate::{perf::CanvasPerfSink, presentation::SessionSnapshot, types::Message},
	iced::{Rectangle, Theme, mouse, widget::canvas},
	std::{sync::Arc, time::Instant},
	tracing::trace_span,
};
pub use {
	geometry::{scene_origin, scene_viewport_size},
	state::CanvasState,
};

#[derive(Debug, Clone)]
pub struct GlyphCanvas {
	pub snapshot: Arc<SessionSnapshot>,
	pub layout_width: f32,
	pub inspect_targets_active: bool,
	pub perf: CanvasPerfSink,
}

impl GlyphCanvas {
	fn inspect_layout(&self) -> Option<&crate::scene::DocumentLayout> {
		// Scene data may already be materialized for debug/perf consumers, but
		// inspect hit testing should still stay off unless the Inspect path is
		// explicitly active.
		self.inspect_targets_active.then_some(())?;
		self.snapshot.scene.as_ref().map(|scene| scene.layout.as_ref())
	}
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
		let max_scroll = max_scroll(bounds, self.snapshot.editor.viewport_metrics, self.layout_width);
		let action = decode_event(
			self.snapshot.mode(),
			state.focused(),
			event,
			self.inspect_layout(),
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

#[cfg(test)]
mod tests {
	use {
		super::GlyphCanvas,
		crate::{
			editor::{EditorMode, EditorViewportMetrics},
			perf::CanvasPerfSink,
			scene::DocumentLayout,
			types::{FontChoice, ShapingChoice, WrapChoice},
		},
		glorp_editor::{
			EditorPresentation, EditorTextLayerState, EditorViewState, ScenePresentation, SessionSnapshot,
			build_buffer, make_font_system, resolve_font_names_from_buffer, scene_config,
		},
		std::sync::Arc,
	};

	fn snapshot(scene: bool) -> SessionSnapshot {
		let editor = EditorPresentation::new(
			1,
			EditorViewportMetrics {
				wrapping: WrapChoice::Word,
				measured_width: 10.0,
				measured_height: 20.0,
			},
			EditorTextLayerState {
				buffer: std::sync::Weak::new(),
				measured_height: 20.0,
			},
			EditorViewState {
				mode: EditorMode::Normal,
				selection: None,
				selection_head: None,
				pointer_anchor: None,
				overlays: Arc::from([]),
				viewport_target: None,
			},
			0,
			0,
			0,
		);
		SessionSnapshot {
			editor,
			scene: scene.then(|| {
				let mut font_system = make_font_system();
				let config = scene_config(
					FontChoice::Monospace,
					ShapingChoice::Auto,
					WrapChoice::Word,
					16.0,
					20.0,
					20.0,
				);
				let buffer = build_buffer(&mut font_system, "abc", config);
				let font_names = resolve_font_names_from_buffer(&font_system, &buffer);
				ScenePresentation::new(
					1,
					Arc::new(DocumentLayout::build("abc", &buffer, config, font_names.as_ref())),
				)
			}),
		}
	}

	#[test]
	fn inspect_layout_depends_on_activation_not_existing_overlays() {
		let canvas = GlyphCanvas {
			snapshot: Arc::new(snapshot(true)),
			layout_width: 20.0,
			inspect_targets_active: true,
			perf: CanvasPerfSink::default(),
		};

		assert!(canvas.inspect_layout().is_some());
	}

	#[test]
	fn inspect_layout_stays_disabled_off_the_inspect_path() {
		let canvas = GlyphCanvas {
			snapshot: Arc::new(snapshot(true)),
			layout_width: 20.0,
			inspect_targets_active: false,
			perf: CanvasPerfSink::default(),
		};

		assert!(canvas.inspect_layout().is_none());
	}
}
