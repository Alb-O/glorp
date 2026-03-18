use {
	glorp_api::GlorpConfig,
	glorp_editor::{SceneConfig, scene_config},
};

#[must_use]
pub const fn scene_config_from_runtime(config: &GlorpConfig, layout_width: f32) -> SceneConfig {
	scene_config(
		config.editor.font,
		config.editor.shaping,
		config.editor.wrapping,
		config.editor.font_size,
		config.editor.line_height,
		layout_width,
	)
}
