use super::{LayoutScene, make_font_system, scene_config};
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
