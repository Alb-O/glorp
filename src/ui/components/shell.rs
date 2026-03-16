use {
	crate::{
		canvas_view::GlyphCanvas,
		editor::{EditorTextLayerState, EditorViewState},
		overlay::OverlayPrimitive,
		overlay_view::{EditorUnderlayLayer, SceneOverlayLayer},
		perf::CanvasPerfSink,
		scene::LayoutScene,
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
	pub(crate) show_baselines: bool,
	pub(crate) show_hitboxes: bool,
}

/// Props for the canvas pane.
pub(crate) struct CanvasPaneProps {
	pub(crate) scene: LayoutScene,
	pub(crate) text_layer: EditorTextLayerState,
	pub(crate) layout_width: f32,
	pub(crate) decorations: CanvasDecorations,
	pub(crate) inspect_overlays: std::sync::Arc<[OverlayPrimitive]>,
	pub(crate) editor: EditorViewState,
	pub(crate) focused: bool,
	pub(crate) scene_revision: u64,
	pub(crate) scroll: Vector,
	pub(crate) perf: CanvasPerfSink,
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
	let backdrop = SceneTextLayer::new(
		props.text_layer.clone(),
		props.editor.clone(),
		props.layout_width,
		props.scroll,
	)
	.backdrop_only()
	.width(Length::Fill)
	.height(Length::Fill);
	let underlay = EditorUnderlayLayer::new(props.editor.clone(), props.scroll, props.perf.clone())
		.width(Length::Fill)
		.height(Length::Fill);
	let text_layer = SceneTextLayer::new(props.text_layer, props.editor.clone(), props.layout_width, props.scroll)
		.text_only()
		.width(Length::Fill)
		.height(Length::Fill);
	let static_scene = StaticSceneLayer::new(
		props.scene.clone(),
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
		props.scene.clone(),
		props.layout_width,
		props.inspect_overlays,
		props.editor.clone(),
		props.focused,
		props.scroll,
		props.perf.clone(),
	)
	.width(Length::Fill)
	.height(Length::Fill);

	let canvas_view = canvas(GlyphCanvas {
		scene: props.scene,
		layout_width: props.layout_width,
		editor: props.editor,
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
