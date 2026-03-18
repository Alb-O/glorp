use {
	crate::{
		canvas_view::GlyphCanvas,
		overlay::OverlayPrimitive,
		overlay_view::{EditorUnderlayLayer, SceneOverlayLayer},
		perf::CanvasPerfSink,
		presentation::SessionSnapshot,
		scene_view::StaticSceneLayer,
		text_view::SceneTextLayer,
		types::{Message, ViewportMessage},
		ui::tokens::{SIDEBAR_WIDTH, surface_style},
	},
	iced::{
		Element, Length, Size, Vector,
		widget::{Stack, canvas, column, container, sensor},
	},
	std::sync::Arc,
};

const STACK_LAYOUT_BREAKPOINT: f32 = 1120.0;
const MIN_CANVAS_WIDTH: f32 = 620.0;

#[derive(Debug, Clone, Copy)]
pub struct CanvasDecorations {
	/// Draw run top and baseline guides over the scene.
	pub show_baselines: bool,
	/// Draw glyph hitbox overlays for inspect mode.
	pub show_hitboxes: bool,
}

/// Immutable inputs for rendering the stacked canvas surface.
///
/// The pane is driven from a single coherent session snapshot.
pub struct CanvasPaneProps {
	/// Shared session snapshot for all canvas sublayers.
	pub snapshot: Arc<SessionSnapshot>,
	/// Current visible layout width after shell sizing and padding.
	pub layout_width: f32,
	/// Optional static scene decorations.
	pub decorations: CanvasDecorations,
	/// Transient inspect overlays derived from hover/selection state.
	pub inspect_overlays: std::sync::Arc<[OverlayPrimitive]>,
	/// Whether inspect hit testing should be active on the canvas path.
	pub inspect_targets_active: bool,
	/// Whether the canvas currently owns keyboard focus.
	pub focused: bool,
	/// Viewport scroll offset in scene coordinates.
	pub scroll: Vector,
	/// Metrics sink shared by the layered canvas widgets.
	pub perf: CanvasPerfSink,
	/// Whether the shell is in stacked mobile layout.
	pub stacked: bool,
}

/// Returns whether the shell should collapse into a stacked layout.
pub fn is_stacked_shell(size: Size) -> bool {
	size.width < STACK_LAYOUT_BREAKPOINT
}

/// The initial sidebar ratio for the wide `pane_grid` shell.
pub fn default_sidebar_ratio() -> f32 {
	SIDEBAR_WIDTH / (SIDEBAR_WIDTH + MIN_CANVAS_WIDTH)
}

/// Builds the stacked shell used below the pane-grid breakpoint.
pub fn view_stacked_shell<'a>(sidebar: Element<'a, Message>, canvas: Element<'a, Message>) -> Element<'a, Message> {
	container(column![canvas, sidebar].spacing(12))
		.padding(16)
		.width(Length::Fill)
		.height(Length::Fill)
		.into()
}

/// Renders the canvas pane inside the shared app surface.
pub fn view_canvas_pane(props: CanvasPaneProps) -> Element<'static, Message> {
	let CanvasPaneProps {
		snapshot,
		layout_width,
		decorations,
		inspect_overlays,
		inspect_targets_active,
		focused,
		scroll,
		perf,
		stacked,
	} = props;
	let backdrop = SceneTextLayer::new(snapshot.clone(), layout_width, scroll)
		.backdrop_only()
		.width(Length::Fill)
		.height(Length::Fill);
	let underlay = EditorUnderlayLayer::new(snapshot.clone(), scroll, perf.clone())
		.width(Length::Fill)
		.height(Length::Fill);
	let text_layer = SceneTextLayer::new(snapshot.clone(), layout_width, scroll)
		.text_only()
		.width(Length::Fill)
		.height(Length::Fill);
	let overlay = SceneOverlayLayer::new(
		snapshot.clone(),
		layout_width,
		inspect_overlays,
		focused,
		scroll,
		perf.clone(),
	)
	.width(Length::Fill)
	.height(Length::Fill);

	let canvas_view = canvas(GlyphCanvas {
		snapshot: snapshot.clone(),
		layout_width,
		inspect_targets_active,
		perf: perf.clone(),
	})
	.width(Length::Fill)
	.height(Length::Fill);

	let static_layer: Option<Element<'static, Message>> = snapshot
		.scene
		.clone()
		.filter(|_| decorations.show_baselines || decorations.show_hitboxes)
		.map(|scene| {
			// The static scene cache only exists for debug geometry. Inspect overlays
			// and the footer can still use the derived scene without paying for this
			// extra layer.
			StaticSceneLayer::new(
				scene,
				layout_width,
				decorations.show_baselines,
				decorations.show_hitboxes,
				scroll,
				perf.clone(),
			)
			.width(Length::Fill)
			.height(Length::Fill)
			.into()
		});
	container(
		sensor(
			Stack::with_children(
				[backdrop.into(), underlay.into(), text_layer.into()]
					.into_iter()
					.chain(static_layer)
					.chain([canvas_view.into(), overlay.into()]),
			)
			.width(Length::Fill)
			.height(Length::Fill),
		)
		.on_resize(|size| Message::Viewport(ViewportMessage::CanvasResized(size))),
	)
	.padding(8)
	.width(Length::Fill)
	.height(if stacked { Length::FillPortion(3) } else { Length::Fill })
	.style(surface_style)
	.into()
}
