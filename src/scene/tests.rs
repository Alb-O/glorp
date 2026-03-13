use super::{LayoutScene, make_font_system, scene_config};
use crate::overlay::{OverlayLayer, OverlayPrimitive, OverlayRectKind};
use crate::types::CanvasTarget;
use crate::types::{FontChoice, RenderMode, ShapingChoice, WrapChoice};

#[test]
fn scene_build_is_stable_for_unicode_replace() {
	let expected = "abX\n漢字\n最後".to_string();
	let config = scene_config(
		FontChoice::SansSerif,
		ShapingChoice::Advanced,
		WrapChoice::Word,
		RenderMode::CanvasOnly,
		22.0,
		30.0,
		320.0,
	);

	let mut rebuilt_font_system = make_font_system();
	let rebuilt = LayoutScene::build(
		&mut rebuilt_font_system,
		expected.clone(),
		config.font_choice,
		config.shaping,
		config.wrapping,
		config.font_size,
		config.line_height,
		config.max_width,
		config.render_mode,
	);

	assert_eq!(rebuilt.text.as_ref(), expected);
	assert!(rebuilt.glyph_count > 0);
	assert!(rebuilt.measured_width > 0.0);
	assert!(rebuilt.measured_height > 0.0);
}

#[test]
fn inspect_overlays_emit_run_and_glyph_primitives() {
	let config = scene_config(
		FontChoice::SansSerif,
		ShapingChoice::Advanced,
		WrapChoice::Word,
		RenderMode::CanvasOnly,
		22.0,
		30.0,
		320.0,
	);
	let mut font_system = make_font_system();
	let scene = LayoutScene::build(
		&mut font_system,
		"alpha beta".to_string(),
		config.font_choice,
		config.shaping,
		config.wrapping,
		config.font_size,
		config.line_height,
		config.max_width,
		config.render_mode,
	);

	let overlays = scene.inspect_overlay_primitives(
		Some(CanvasTarget::Run(0)),
		Some(CanvasTarget::Glyph {
			run_index: 0,
			glyph_index: 0,
		}),
		config.max_width,
		true,
	);

	assert!(overlays.iter().any(|primitive| matches!(
		primitive,
		OverlayPrimitive::Rect {
			kind: OverlayRectKind::InspectRunHover,
			layer: OverlayLayer::OverText,
			..
		}
	)));
	assert!(overlays.iter().any(|primitive| matches!(
		primitive,
		OverlayPrimitive::Rect {
			kind: OverlayRectKind::InspectGlyphSelected,
			layer: OverlayLayer::OverText,
			..
		}
	)));
	assert!(overlays.iter().any(|primitive| matches!(
		primitive,
		OverlayPrimitive::Rect {
			kind: OverlayRectKind::InspectGlyphHitboxSelected,
			layer: OverlayLayer::OverText,
			..
		}
	)));
}

#[test]
fn inspect_overlays_fall_back_to_clusters_without_lazy_runs() {
	let config = scene_config(
		FontChoice::SansSerif,
		ShapingChoice::Advanced,
		WrapChoice::Word,
		RenderMode::CanvasOnly,
		22.0,
		30.0,
		320.0,
	);
	let mut font_system = make_font_system();
	let scene = LayoutScene::build(
		&mut font_system,
		"alpha beta".to_string(),
		config.font_choice,
		config.shaping,
		config.wrapping,
		config.font_size,
		config.line_height,
		config.max_width,
		config.render_mode,
	);

	let overlays = scene.inspect_overlay_primitives(
		None,
		Some(CanvasTarget::Glyph {
			run_index: 0,
			glyph_index: 0,
		}),
		config.max_width,
		false,
	);

	assert_eq!(overlays.len(), 1);
	assert!(matches!(
		&overlays[0],
		OverlayPrimitive::Rect {
			kind: OverlayRectKind::InspectGlyphSelected,
			layer: OverlayLayer::OverText,
			..
		}
	));
}
