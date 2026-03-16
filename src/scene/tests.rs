use {
	super::{DocumentLayout, make_font_system, scene_config},
	crate::{
		overlay::{OverlayLayer, OverlayRectKind},
		types::{CanvasTarget, FontChoice, ShapingChoice, WrapChoice},
	},
};

#[test]
fn document_layout_build_is_stable_for_unicode_replace() {
	let expected = "abX\n漢字\n最後".to_string();
	let config = scene_config(
		FontChoice::SansSerif,
		ShapingChoice::Advanced,
		WrapChoice::Word,
		22.0,
		30.0,
		320.0,
	);

	let mut font_system = make_font_system();
	let buffer = super::build_buffer(&mut font_system, &expected, config);
	let font_names = super::resolve_font_names_from_buffer(&font_system, &buffer);
	let layout = DocumentLayout::build(&expected, &buffer, config, font_names.as_ref());

	assert_eq!(layout.text.as_ref(), expected);
	assert!(layout.glyph_count > 0);
	assert!(layout.measured_width > 0.0);
	assert!(layout.measured_height > 0.0);
}

#[test]
fn inspect_overlays_emit_run_and_cluster_primitives() {
	let config = scene_config(
		FontChoice::SansSerif,
		ShapingChoice::Advanced,
		WrapChoice::Word,
		22.0,
		30.0,
		320.0,
	);
	let mut font_system = make_font_system();
	let buffer = super::build_buffer(&mut font_system, "alpha beta", config);
	let font_names = super::resolve_font_names_from_buffer(&font_system, &buffer);
	let layout = DocumentLayout::build("alpha beta", &buffer, config, font_names.as_ref());

	let overlays = layout.inspect_overlay_primitives(
		Some(CanvasTarget::Run(0)),
		Some(CanvasTarget::Cluster(0)),
		config.max_width,
		true,
	);

	assert!(overlays.iter().any(|primitive| matches!(
		(primitive.kind, primitive.layer),
		(OverlayRectKind::InspectRunHover, OverlayLayer::OverText)
	)));
	assert!(overlays.iter().any(|primitive| matches!(
		(primitive.kind, primitive.layer),
		(OverlayRectKind::InspectGlyphSelected, OverlayLayer::OverText)
	)));
	assert!(overlays.iter().any(|primitive| matches!(
		(primitive.kind, primitive.layer),
		(OverlayRectKind::InspectGlyphHitboxSelected, OverlayLayer::OverText)
	)));
}

#[test]
fn cluster_target_details_are_rebuilt_without_cache_layer() {
	let config = scene_config(
		FontChoice::SansSerif,
		ShapingChoice::Advanced,
		WrapChoice::Word,
		22.0,
		30.0,
		320.0,
	);
	let mut font_system = make_font_system();
	let buffer = super::build_buffer(&mut font_system, "alpha beta", config);
	let font_names = super::resolve_font_names_from_buffer(&font_system, &buffer);
	let layout = DocumentLayout::build("alpha beta", &buffer, config, font_names.as_ref());

	let details = layout
		.target_details(Some(CanvasTarget::Cluster(0)))
		.expect("cluster details should exist");

	assert!(details.contains("kind: cluster"));
	assert!(details.contains("cluster index: 0"));
}
