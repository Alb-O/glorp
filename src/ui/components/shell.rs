use {
	crate::{
		canvas_view::GlyphCanvas,
		overlay::OverlayPrimitive,
		overlay_view::{EditorUnderlayLayer, SceneOverlayLayer},
		perf::CanvasPerfSink,
		presentation::{DerivedScenePresentation, EditorPresentation},
		scene_view::StaticSceneLayer,
		text_view::SceneTextLayer,
		types::{Message, ViewportMessage},
		ui::tokens::{SIDEBAR_WIDTH, surface_style},
	},
	iced::{
		Element, Length, Size, Vector,
		widget::{Stack, canvas, column, container, sensor},
	},
};

const STACK_LAYOUT_BREAKPOINT: f32 = 1120.0;
const MIN_CANVAS_WIDTH: f32 = 620.0;

#[derive(Debug, Clone, Copy)]
pub(crate) struct CanvasDecorations {
	/// Draw run top and baseline guides over the scene.
	pub(crate) show_baselines: bool,
	/// Draw glyph hitbox overlays for inspect mode.
	pub(crate) show_hitboxes: bool,
}

/// Immutable inputs for rendering the stacked canvas surface.
///
/// The pane is driven from the hot editor presentation plus an optional lazily
/// materialized scene snapshot for inspect/perf/debug consumers.
pub(crate) struct CanvasPaneProps {
	/// Shared hot-path presentation snapshot for all canvas sublayers.
	pub(crate) editor_presentation: EditorPresentation,
	/// Optional derived scene snapshot for inspect/perf/debug consumers.
	pub(crate) derived_scene: Option<DerivedScenePresentation>,
	/// Current visible layout width after shell sizing and padding.
	pub(crate) layout_width: f32,
	/// Optional static scene decorations.
	pub(crate) decorations: CanvasDecorations,
	/// Transient inspect overlays derived from hover/selection state.
	pub(crate) inspect_overlays: std::sync::Arc<[OverlayPrimitive]>,
	/// Whether inspect hit testing should be active on the canvas path.
	pub(crate) inspect_targets_active: bool,
	/// Whether the canvas currently owns keyboard focus.
	pub(crate) focused: bool,
	/// Revision key for static-scene cache invalidation.
	pub(crate) scene_revision: u64,
	/// Viewport scroll offset in scene coordinates.
	pub(crate) scroll: Vector,
	/// Metrics sink shared by the layered canvas widgets.
	pub(crate) perf: CanvasPerfSink,
	/// Whether the shell is in stacked mobile layout.
	pub(crate) stacked: bool,
}

/// Returns whether the shell should collapse into a stacked layout.
pub(crate) fn is_stacked_shell(size: Size) -> bool {
	size.width < STACK_LAYOUT_BREAKPOINT
}

/// The initial sidebar ratio for the wide `pane_grid` shell.
pub(crate) fn default_sidebar_ratio() -> f32 {
	SIDEBAR_WIDTH / (SIDEBAR_WIDTH + MIN_CANVAS_WIDTH)
}

/// Builds the stacked shell used below the pane-grid breakpoint.
pub(crate) fn view_stacked_shell<'a>(
	sidebar: Element<'a, Message>, canvas: Element<'a, Message>,
) -> Element<'a, Message> {
	container(column![canvas, sidebar].spacing(12))
		.padding(16)
		.width(Length::Fill)
		.height(Length::Fill)
		.into()
}

/// Renders the canvas pane inside the shared app surface.
pub(crate) fn view_canvas_pane(props: CanvasPaneProps) -> Element<'static, Message> {
	let CanvasPaneProps {
		editor_presentation,
		derived_scene,
		layout_width,
		decorations,
		inspect_overlays,
		inspect_targets_active,
		focused,
		scene_revision,
		scroll,
		perf,
		stacked,
	} = props;
	let backdrop = SceneTextLayer::new(editor_presentation.clone(), layout_width, scroll)
		.backdrop_only()
		.width(Length::Fill)
		.height(Length::Fill);
	let underlay = EditorUnderlayLayer::new(editor_presentation.clone(), scroll, perf.clone())
		.width(Length::Fill)
		.height(Length::Fill);
	let text_layer = SceneTextLayer::new(editor_presentation.clone(), layout_width, scroll)
		.text_only()
		.width(Length::Fill)
		.height(Length::Fill);
	let overlay = SceneOverlayLayer::new(
		editor_presentation.clone(),
		derived_scene.clone(),
		layout_width,
		inspect_overlays,
		focused,
		scroll,
		perf.clone(),
	)
	.width(Length::Fill)
	.height(Length::Fill);

	let canvas_view = canvas(GlyphCanvas {
		editor_presentation,
		derived_scene: derived_scene.clone(),
		layout_width,
		inspect_targets_active,
		perf: perf.clone(),
	})
	.width(Length::Fill)
	.height(Length::Fill);

	let static_layer = derived_scene
		.filter(|_| decorations.show_baselines || decorations.show_hitboxes)
		.map(|derived_scene| {
			// The static scene cache only exists for debug geometry. Inspect overlays
			// and the footer can still use the derived scene without paying for this
			// extra layer.
			StaticSceneLayer::new(
				derived_scene,
				layout_width,
				decorations.show_baselines,
				decorations.show_hitboxes,
				scene_revision,
				scroll,
				perf.clone(),
			)
			.width(Length::Fill)
			.height(Length::Fill)
			.into()
		});
	let children = [backdrop.into(), underlay.into(), text_layer.into()]
		.into_iter()
		.chain(static_layer)
		.chain([canvas_view.into(), overlay.into()])
		.collect::<Vec<_>>();

	container(
		sensor(Stack::with_children(children).width(Length::Fill).height(Length::Fill))
			.on_resize(|size| Message::Viewport(ViewportMessage::CanvasResized(size))),
	)
	.padding(8)
	.width(Length::Fill)
	.height(if stacked { Length::FillPortion(3) } else { Length::Fill })
	.style(surface_style)
	.into()
}
