mod geometry;
mod input;
mod state;

use {
	self::{geometry::max_scroll, input::decode_event},
	crate::{
		perf::CanvasPerfSink,
		presentation::{DerivedScenePresentation, EditorPresentation},
		types::Message,
	},
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
	pub(crate) editor_presentation: EditorPresentation,
	pub(crate) derived_scene: Option<DerivedScenePresentation>,
	pub(crate) layout_width: f32,
	pub(crate) inspect_targets_active: bool,
	pub(crate) perf: CanvasPerfSink,
}

impl GlyphCanvas {
	fn inspect_layout(&self) -> Option<&crate::scene::DocumentLayout> {
		// Scene data may already be materialized for debug/perf consumers, but
		// inspect hit testing should still stay off unless the Inspect path is
		// explicitly active.
		self.inspect_targets_active
			.then(|| self.derived_scene.as_ref().map(|scene| scene.layout.as_ref()))
			.flatten()
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
		let max_scroll = max_scroll(bounds, self.editor_presentation.viewport_metrics, self.layout_width);
		let action = decode_event(
			self.editor_presentation.mode(),
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
			editor::{EditorMode, EditorTextLayerState, EditorViewState, EditorViewportMetrics},
			perf::CanvasPerfSink,
			presentation::{DerivedScenePresentation, EditorPresentation},
			scene::{DocumentLayout, DocumentLayoutTestSpec},
			types::WrapChoice,
		},
		std::sync::Arc,
	};

	fn editor_presentation() -> EditorPresentation {
		EditorPresentation::new(
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
		)
	}

	fn derived_scene() -> DerivedScenePresentation {
		DerivedScenePresentation::new(
			1,
			Arc::new(DocumentLayout::new_for_test(DocumentLayoutTestSpec {
				text: Arc::<str>::from("abc"),
				wrapping: WrapChoice::Word,
				max_width: 20.0,
				measured_width: 20.0,
				measured_height: 20.0,
				glyph_count: 0,
				font_count: 0,
				runs: Vec::new(),
				clusters: Vec::new(),
			})),
		)
	}

	#[test]
	fn inspect_layout_depends_on_activation_not_existing_overlays() {
		let canvas = GlyphCanvas {
			editor_presentation: editor_presentation(),
			derived_scene: Some(derived_scene()),
			layout_width: 20.0,
			inspect_targets_active: true,
			perf: CanvasPerfSink::default(),
		};

		assert!(canvas.inspect_layout().is_some());
	}

	#[test]
	fn inspect_layout_stays_disabled_off_the_inspect_path() {
		let canvas = GlyphCanvas {
			editor_presentation: editor_presentation(),
			derived_scene: Some(derived_scene()),
			layout_width: 20.0,
			inspect_targets_active: false,
			perf: CanvasPerfSink::default(),
		};

		assert!(canvas.inspect_layout().is_none());
	}
}
