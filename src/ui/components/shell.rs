use {
	crate::{
		canvas_view::GlyphCanvas,
		overlay::OverlayPrimitive,
		overlay_view::{EditorUnderlayLayer, SceneOverlayLayer},
		perf::CanvasPerfSink,
		presentation::DocumentPresentation,
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
/// The pane is driven from a single [`DocumentPresentation`] plus a small set
/// of view-local flags such as scroll position and focus state.
pub(crate) struct CanvasPaneProps {
	/// Shared presentation snapshot for all canvas sublayers.
	pub(crate) presentation: DocumentPresentation,
	/// Current visible layout width after shell sizing and padding.
	pub(crate) layout_width: f32,
	/// Optional static scene decorations.
	pub(crate) decorations: CanvasDecorations,
	/// Transient inspect overlays derived from hover/selection state.
	pub(crate) inspect_overlays: std::sync::Arc<[OverlayPrimitive]>,
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
	let backdrop = SceneTextLayer::new(props.presentation.clone(), props.layout_width, props.scroll)
		.backdrop_only()
		.width(Length::Fill)
		.height(Length::Fill);
	let underlay = EditorUnderlayLayer::new(props.presentation.clone(), props.scroll, props.perf.clone())
		.width(Length::Fill)
		.height(Length::Fill);
	let text_layer = SceneTextLayer::new(props.presentation.clone(), props.layout_width, props.scroll)
		.text_only()
		.width(Length::Fill)
		.height(Length::Fill);
	let static_scene = StaticSceneLayer::new(
		props.presentation.clone(),
		props.layout_width,
		props.decorations.show_baselines,
		props.decorations.show_hitboxes,
		props.scene_revision,
		props.scroll,
		props.perf.clone(),
	)
	.width(Length::Fill)
	.height(Length::Fill);
	let overlay = SceneOverlayLayer::new(
		props.presentation.clone(),
		props.layout_width,
		props.inspect_overlays,
		props.focused,
		props.scroll,
		props.perf.clone(),
	)
	.width(Length::Fill)
	.height(Length::Fill);

	let canvas_view = canvas(GlyphCanvas {
		presentation: props.presentation,
		layout_width: props.layout_width,
		perf: props.perf,
	})
	.width(Length::Fill)
	.height(Length::Fill);

	container(
		sensor(
			Stack::with_children([
				backdrop.into(),
				underlay.into(),
				text_layer.into(),
				static_scene.into(),
				canvas_view.into(),
				overlay.into(),
			])
			.width(Length::Fill)
			.height(Length::Fill),
		)
		.on_resize(|size| Message::Viewport(ViewportMessage::CanvasResized(size))),
	)
	.padding(8)
	.width(Length::Fill)
	.height(if props.stacked {
		Length::FillPortion(3)
	} else {
		Length::Fill
	})
	.style(surface_style)
	.into()
}
