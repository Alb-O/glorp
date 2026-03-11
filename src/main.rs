//! Low-level text and glyph playground.
//!
//! This example sits between `iced` widget code and the text stack underneath
//! it. It is intentionally not a full custom renderer. The point is to expose
//! the seams:
//!
//! - edit text locally
//! - shape and lay it out with `cosmic-text`
//! - inspect runs and glyphs directly
//! - compare that data against `canvas::Text`
//! - draw vendored glyph outlines from `swash`
//!
//! # Upstream map
//!
//! ## Already adapted locally
//!
//! These upstream pieces are copied or re-expressed in this crate.
//!
//! 1. Font to `cosmic-text::Attrs`
//!    - Upstream: `iced_graphics::text`
//!    - Local: `scene::to_attributes`, `scene::to_family`,
//!      `scene::to_weight`, `scene::to_stretch`, `scene::to_style`
//!    - Why: this keeps our shaping inputs close to what `iced` itself uses.
//!
//! 2. Swash outline traversal
//!    - Upstream: `iced_graphics::geometry::text::Text::draw_with`
//!    - Local: outline extraction in `scene::LayoutScene::build`
//!    - Why: this is the core of the local outline rendering mode.
//!
//! ## Still external and vital
//!
//! These are still runtime-defining and are not vendored here.
//!
//! 1. Canvas widget event model
//!    - Upstream: `iced_widget::canvas::program`, `iced_widget::action`
//!    - Affects: `canvas_view::GlyphCanvas` interaction and message publishing
//!    - Why it matters: hover/click behavior still follows Iced's widget
//!      runtime contracts.
//!
//! 2. `canvas::Text` renderer path
//!    - Upstream: `iced_wgpu::geometry`, `iced_wgpu::text`,
//!      `iced_graphics::text::cache`
//!    - Affects: the blue `canvas::Text` overlay
//!    - Why it matters: caching, clipping, atlas upload, and final GPU text
//!      rendering are still upstream-owned.
//!
//! 3. `cosmic-text` layout model
//!    - Upstream: `cosmic_text::buffer`, `cosmic_text::layout`
//!    - Affects: `scene::RunInfo`, `scene::GlyphInfo`, `glyph.physical(...)`
//!    - Why it matters: our hit-testing, dumps, and glyph boxes all depend on
//!      these structures and their semantics.
//!
//! 4. `cosmic-text::FontSystem` fallback behavior
//!    - Upstream: `cosmic_text::font::system`
//!    - Affects: actual face resolution and fallback
//!    - Why it matters: `scene::make_font_system()` only augments the
//!      database; it does not replace fallback policy.
//!
//! ## Best snipe targets
//!
//! If we want to reduce upstream dependency in order of leverage:
//!
//! 1. `canvas::Text` renderer path
//! 2. `cosmic-text` layout/run abstraction
//! 3. `cosmic-text` font fallback policy
//! 4. Canvas event/action API
//!
//! ## Not worth adapting yet
//!
//! - Font to `Attrs` conversion: already local
//! - Swash outline traversal: already local
//! - Hover/click hit-testing: already local
mod app;
mod canvas_view;
mod scene;
mod types;
mod ui;

use iced::{Font, Theme};

use crate::app::Playground;

pub fn run() -> iced::Result {
	let settings = iced::Settings {
		default_font: Font::with_name("Noto Sans CJK SC"),
		..Default::default()
	};

	iced::application(Playground::new, Playground::update, Playground::view)
		.theme(app_theme)
		.settings(settings)
		.run()
}

fn app_theme(_playground: &Playground) -> Theme {
	Theme::TokyoNightStorm
}

#[allow(dead_code)]
fn main() -> iced::Result {
	run()
}
