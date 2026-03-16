//! Editor-first text layout and glyph inspection app.
//!
//! This example sits between `iced` widget code and the text stack underneath
//! it. It is intentionally not a full custom renderer. The point is to expose
//! the seams:
//!
//! - edit text locally
//! - shape and lay it out with `cosmic-text`
//! - inspect runs and glyphs directly
//! - compare that data against Iced's paragraph renderer
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
//! - Hover/click hit-testing: already local
mod app;
mod canvas_view;
mod editor;
mod headless_perf;
mod overlay;
mod overlay_view;
mod perf;
mod presentation;
mod scene;
mod scene_view;
mod telemetry;
mod text_view;
mod types;
mod ui;

pub use app::EditorApp;
use {
	iced::{Font, Theme},
	std::process::ExitCode,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeadlessScenario {
	Default,
	Tall,
	TallInspect,
	TallPerf,
}

impl HeadlessScenario {
	pub const ALL: [Self; 4] = [Self::Default, Self::Tall, Self::TallInspect, Self::TallPerf];

	#[must_use]
	pub fn label(self) -> &'static str {
		match self {
			Self::Default => "default",
			Self::Tall => "tall",
			Self::TallInspect => "tall-inspect",
			Self::TallPerf => "tall-perf",
		}
	}

	#[must_use]
	pub fn parse_label(label: &str) -> Option<Self> {
		Self::ALL.into_iter().find(|scenario| scenario.label() == label)
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerfScenario {
	Default,
	Tall,
	TallInspect,
	TallPerf,
	IncrementalTyping,
	MotionSweep,
	ResizeReflow,
	InspectInteraction,
}

impl PerfScenario {
	pub const ALL: [Self; 8] = [
		Self::Default,
		Self::Tall,
		Self::TallInspect,
		Self::TallPerf,
		Self::IncrementalTyping,
		Self::MotionSweep,
		Self::ResizeReflow,
		Self::InspectInteraction,
	];

	#[must_use]
	pub fn label(self) -> &'static str {
		match self {
			Self::Default => "default",
			Self::Tall => "tall",
			Self::TallInspect => "tall-inspect",
			Self::TallPerf => "tall-perf",
			Self::IncrementalTyping => "incremental-typing",
			Self::MotionSweep => "motion-sweep",
			Self::ResizeReflow => "resize-reflow",
			Self::InspectInteraction => "inspect-interaction",
		}
	}

	#[must_use]
	pub fn parse_label(label: &str) -> Option<Self> {
		Self::ALL.into_iter().find(|scenario| scenario.label() == label)
	}

	#[must_use]
	pub fn driver(self) -> &'static str {
		match self {
			Self::Default | Self::Tall | Self::TallInspect | Self::TallPerf => "steady-render",
			Self::IncrementalTyping | Self::MotionSweep | Self::ResizeReflow | Self::InspectInteraction => {
				"scripted-update-render"
			}
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

	#[must_use]
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

/// Runs the interactive editor application.
///
/// # Errors
///
/// Returns any startup or runtime error reported by `iced`.
pub fn run() -> iced::Result {
	telemetry::init_tracing();

	let settings = iced::Settings {
		default_font: Font::new("Noto Sans CJK SC"),
		..Default::default()
	};

	iced::application(EditorApp::new, EditorApp::update, EditorApp::view)
		.subscription(EditorApp::subscription)
		.theme(app_theme)
		.settings(settings)
		.run()
}

#[must_use]
pub fn main_entry() -> ExitCode {
	if let Some(code) = headless_perf::run_from_env() {
		code
	} else {
		match run() {
			Ok(()) => ExitCode::SUCCESS,
			Err(error) => {
				eprintln!("{error}");
				ExitCode::FAILURE
			}
		}
	}
}

pub fn init_tracing() {
	telemetry::init_tracing();
}

fn app_theme(_app: &EditorApp) -> Theme {
	Theme::TokyoNightStorm
}
