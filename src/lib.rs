//! Low-level text and glyph playground.
//!
//! This example sits between `iced` widget code and the text stack underneath
//! it. It is intentionally not a full custom renderer. The point is to expose
//! the seams:
//!
//! - edit text locally
//! - shape and lay it out with `cosmic-text`
//! - inspect runs and glyphs directly
//! - compare that data against Iced's paragraph renderer
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
//! 2. Paragraph renderer path
//!    - Upstream: `iced_graphics::text`, `iced_wgpu::text`
//!    - Affects: the document text layer in `text_view`
//!    - Why it matters: clipping, atlas upload, and final GPU text rendering
//!      are still upstream-owned.
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
//! 1. Paragraph renderer path
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
mod editor;
mod overlay;
mod perf;
mod scene;
mod telemetry;
mod text_view;
mod types;
mod ui;

pub use app::Playground;
use iced::{Font, Theme};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeadlessScenario {
	Default,
	Tall,
	TallInspect,
	TallPerf,
}

impl HeadlessScenario {
	pub const ALL: [Self; 4] = [Self::Default, Self::Tall, Self::TallInspect, Self::TallPerf];

	pub fn label(self) -> &'static str {
		match self {
			Self::Default => "default",
			Self::Tall => "tall",
			Self::TallInspect => "tall-inspect",
			Self::TallPerf => "tall-perf",
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeadlessScriptScenario {
	LargePaste,
	IncrementalTyping,
	IncrementalLineBreaks,
	UndoRedoBurst,
	BackspaceBurst,
	DeleteForwardBurst,
	MotionSweep,
	PointerSelectionSweep,
	ResizeReflowSweep,
	InspectInteractionSweep,
}

impl HeadlessScriptScenario {
	pub const ALL: [Self; 10] = [
		Self::LargePaste,
		Self::IncrementalTyping,
		Self::IncrementalLineBreaks,
		Self::UndoRedoBurst,
		Self::BackspaceBurst,
		Self::DeleteForwardBurst,
		Self::MotionSweep,
		Self::PointerSelectionSweep,
		Self::ResizeReflowSweep,
		Self::InspectInteractionSweep,
	];

	pub fn label(self) -> &'static str {
		match self {
			Self::LargePaste => "large-paste",
			Self::IncrementalTyping => "incremental-typing",
			Self::IncrementalLineBreaks => "incremental-line-breaks",
			Self::UndoRedoBurst => "undo-redo-burst",
			Self::BackspaceBurst => "backspace-burst",
			Self::DeleteForwardBurst => "delete-forward-burst",
			Self::MotionSweep => "motion-sweep",
			Self::PointerSelectionSweep => "pointer-selection-sweep",
			Self::ResizeReflowSweep => "resize-reflow-sweep",
			Self::InspectInteractionSweep => "inspect-interaction-sweep",
		}
	}
}

pub fn run() -> iced::Result {
	telemetry::init_tracing();

	let settings = iced::Settings {
		default_font: Font::with_name("Noto Sans CJK SC"),
		..Default::default()
	};

	iced::application(Playground::new, Playground::update, Playground::view)
		.subscription(Playground::subscription)
		.theme(app_theme)
		.settings(settings)
		.run()
}

pub fn init_tracing() {
	telemetry::init_tracing();
}

fn app_theme(_playground: &Playground) -> Theme {
	Theme::TokyoNightStorm
}
