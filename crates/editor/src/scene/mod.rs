//! Text layout and scene materialization.
//!
//! This is the other major internal seam inside `glorp_editor`. It converts
//! document text plus scene config into measurable layout artifacts that higher
//! layers can render, inspect, and hit-test.
//!
//! # Responsibilities
//!
//! - font-system setup and font-name resolution
//! - buffer construction for `cosmic_text`
//! - layout runs, clusters, caret metrics, and byte-order helpers
//! - geometry answers used by editor interaction and rendering
//!
//! # Non-responsibilities
//!
//! - deciding which edit should happen
//! - storing undo/redo history
//! - transport-safe public protocol design
//! - GUI widget composition
//!
//! # Why this seam exists
//!
//! Layout is expensive, stateful, and implementation-specific. By keeping it in
//! a dedicated module, the rest of the codebase can depend on stable read models
//! such as [`DocumentLayout`] and [`SceneConfig`] without coupling public
//! semantics to `cosmic_text` details.

mod build;
mod data;
mod font;
mod geometry;
mod inspect;
#[cfg(test)]
mod tests;
mod text;

use {
	crate::types::{FontChoice, FontChoiceExt, ShapingChoice, WrapChoice},
	iced::Font,
	std::sync::Arc,
};

#[cfg(test)]
pub use self::build::DocumentLayoutTestSpec;
pub use self::{
	build::resolve_font_names_from_buffer,
	data::{LayoutCaretMetrics, LayoutCluster, LayoutRun},
	font::{build_buffer, make_font_system, scene_config},
	text::{debug_range, line_byte_offsets},
};

pub type FontNameMap = Arc<[(cosmic_text::fontdb::ID, Arc<str>)]>;

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct SceneConfig {
	pub font_choice: FontChoice,
	pub shaping: ShapingChoice,
	pub wrapping: WrapChoice,
	pub font_size: f32,
	pub line_height: f32,
	pub max_width: f32,
}

impl SceneConfig {
	#[must_use]
	pub fn font(self) -> Font {
		self.font_choice.to_iced_font()
	}
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DocumentLayout {
	pub text: Arc<str>,
	pub wrapping: WrapChoice,
	pub max_width: f32,
	pub measured_width: f32,
	pub measured_height: f32,
	pub glyph_count: usize,
	pub cluster_count: usize,
	pub font_count: usize,
	pub runs: Arc<[LayoutRun]>,
	pub clusters: Arc<[LayoutCluster]>,
	pub line_byte_offsets: Arc<[usize]>,
	byte_order: Arc<[usize]>,
	pub warnings: Arc<[String]>,
}
