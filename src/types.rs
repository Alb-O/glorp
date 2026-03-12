use iced::Font;
use iced::Point;
use iced::Size;
use iced::Vector;
use iced::advanced::text::Shaping;
use iced::advanced::text::Wrapping;
use iced::time::Instant;
use iced::widget::pane_grid;

use std::fmt::{self, Display};

use crate::editor::EditorCommand;

#[derive(Debug, Clone)]
pub(crate) enum Message {
	LoadPreset(SamplePreset),
	FontSelected(FontChoice),
	ShapingSelected(ShapingChoice),
	WrappingSelected(WrapChoice),
	RenderModeSelected(RenderMode),
	FontSizeChanged(f32),
	LineHeightChanged(f32),
	ShowBaselinesChanged(bool),
	ShowHitboxesChanged(bool),
	SelectSidebarTab(SidebarTab),
	PerfTick(Instant),
	CanvasViewportResized(Size),
	ResizeTick(Instant),
	CanvasHovered(Option<CanvasTarget>),
	CanvasScrollChanged(Vector),
	CanvasPressed {
		target: Option<CanvasTarget>,
		position: Point,
		double_click: bool,
	},
	CanvasDragged(Point),
	CanvasReleased,
	PaneResized(pane_grid::ResizeEvent),
	EditorCommand(EditorCommand),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SidebarTab {
	Controls,
	Inspect,
	Perf,
}

impl SidebarTab {
	pub(crate) const ALL: [Self; 3] = [Self::Controls, Self::Inspect, Self::Perf];
}

impl Display for SidebarTab {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let label = match self {
			Self::Controls => "Controls",
			Self::Inspect => "Inspect",
			Self::Perf => "Perf",
		};

		f.write_str(label)
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CanvasTarget {
	Run(usize),
	Glyph { run_index: usize, glyph_index: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SamplePreset {
	Tall,
	Mixed,
	Rust,
	Ligatures,
	Arabic,
	Cjk,
	Emoji,
	Custom,
}

impl SamplePreset {
	pub(crate) const ALL: [SamplePreset; 8] = [
		SamplePreset::Tall,
		SamplePreset::Mixed,
		SamplePreset::Rust,
		SamplePreset::Ligatures,
		SamplePreset::Arabic,
		SamplePreset::Cjk,
		SamplePreset::Emoji,
		SamplePreset::Custom,
	];

	pub(crate) fn text(self) -> &'static str {
		match self {
			SamplePreset::Tall => concat!(
				"chapter 01: office affine ffi ffl fj\n",
				"chapter 02: 漢字カタカナ and Latin in one lane\n",
				"chapter 03: السلام عليكم مع سطور إضافية\n",
				"chapter 04: emoji 🙂🚀👩‍💻 over baseline checks\n",
				"chapter 05: fjord buffer glyph wrap probe\n",
				"chapter 06: 日本語の行送りと混在テキスト\n",
				"chapter 07: bidi mix -> abc אבג 123\n",
				"chapter 08: outline fallback and font fallback\n",
				"chapter 09: ligatures office official affluent\n",
				"chapter 10: accents cafe café caffè caﬀe\n",
				"chapter 11: ASCII rulers 0123456789\n",
				"chapter 12: more emoji 🧪🧭🌊🛰️\n",
				"chapter 13: the quick brown fox scroll probe\n",
				"chapter 14: glyph boxes should keep coming\n",
				"chapter 15: this canvas now has vertical runway\n",
				"chapter 16: Arabic مرحبا بالعالم مرة ثانية\n",
				"chapter 17: kana かなカナ漢字ひらがな\n",
				"chapter 18: source editing should still work\n",
				"chapter 19: swipe or wheel to pan the scene\n",
				"chapter 20: end marker"
			),
			SamplePreset::Mixed => "office affine ffi ffl\n漢字カタカナ and Latin\nالسلام عليكم\nemoji 🙂🚀👩‍💻",
			SamplePreset::Rust => "fn main() {\n    println!(\"ffi -> office -> 汉字\");\n}\n",
			SamplePreset::Ligatures => "office affine final fluff ffi ffl fj",
			SamplePreset::Arabic => "السلام عليكم\nمرحبا بالعالم",
			SamplePreset::Cjk => "漢字かなカナ\n混在テキスト with ASCII",
			SamplePreset::Emoji => "🙂🚀👩‍💻 text + emoji fallback",
			SamplePreset::Custom => "",
		}
	}
}

impl Display for SamplePreset {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			SamplePreset::Tall => write!(f, "Tall"),
			SamplePreset::Mixed => write!(f, "Mixed"),
			SamplePreset::Rust => write!(f, "Rust"),
			SamplePreset::Ligatures => write!(f, "Ligatures"),
			SamplePreset::Arabic => write!(f, "Arabic"),
			SamplePreset::Cjk => write!(f, "CJK"),
			SamplePreset::Emoji => write!(f, "Emoji"),
			SamplePreset::Custom => write!(f, "Custom"),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FontChoice {
	JetBrainsMono,
	Monospace,
	NotoSansCjk,
	SansSerif,
}

impl FontChoice {
	pub(crate) const ALL: [FontChoice; 4] = [
		FontChoice::JetBrainsMono,
		FontChoice::Monospace,
		FontChoice::NotoSansCjk,
		FontChoice::SansSerif,
	];

	pub(crate) fn to_iced_font(self) -> Font {
		match self {
			FontChoice::JetBrainsMono => Font::with_name("JetBrains Mono"),
			FontChoice::Monospace => Font::MONOSPACE,
			FontChoice::NotoSansCjk => Font::with_name("Noto Sans CJK SC"),
			FontChoice::SansSerif => Font::DEFAULT,
		}
	}
}

impl Display for FontChoice {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			FontChoice::JetBrainsMono => write!(f, "JetBrains Mono"),
			FontChoice::Monospace => write!(f, "Monospace family"),
			FontChoice::NotoSansCjk => write!(f, "Noto Sans CJK SC"),
			FontChoice::SansSerif => write!(f, "Sans Serif family"),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShapingChoice {
	Auto,
	Basic,
	Advanced,
}

impl ShapingChoice {
	pub(crate) const ALL: [ShapingChoice; 3] = [Self::Auto, Self::Basic, Self::Advanced];

	pub(crate) fn to_iced(self) -> Shaping {
		match self {
			ShapingChoice::Auto => Shaping::Auto,
			ShapingChoice::Basic => Shaping::Basic,
			ShapingChoice::Advanced => Shaping::Advanced,
		}
	}

	pub(crate) fn to_cosmic(self, text: &str) -> cosmic_text::Shaping {
		match self {
			ShapingChoice::Auto => {
				if text.is_ascii() {
					cosmic_text::Shaping::Basic
				} else {
					cosmic_text::Shaping::Advanced
				}
			}
			ShapingChoice::Basic => cosmic_text::Shaping::Basic,
			ShapingChoice::Advanced => cosmic_text::Shaping::Advanced,
		}
	}
}

impl Display for ShapingChoice {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			ShapingChoice::Auto => write!(f, "Auto"),
			ShapingChoice::Basic => write!(f, "Basic"),
			ShapingChoice::Advanced => write!(f, "Advanced"),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WrapChoice {
	None,
	Word,
	Glyph,
	WordOrGlyph,
}

impl WrapChoice {
	pub(crate) const ALL: [WrapChoice; 4] = [Self::None, Self::Word, Self::Glyph, Self::WordOrGlyph];

	pub(crate) fn to_iced(self) -> Wrapping {
		match self {
			WrapChoice::None => Wrapping::None,
			WrapChoice::Word => Wrapping::Word,
			WrapChoice::Glyph => Wrapping::Glyph,
			WrapChoice::WordOrGlyph => Wrapping::WordOrGlyph,
		}
	}

	pub(crate) fn to_cosmic(self) -> cosmic_text::Wrap {
		match self {
			WrapChoice::None => cosmic_text::Wrap::None,
			WrapChoice::Word => cosmic_text::Wrap::Word,
			WrapChoice::Glyph => cosmic_text::Wrap::Glyph,
			WrapChoice::WordOrGlyph => cosmic_text::Wrap::WordOrGlyph,
		}
	}
}

impl Display for WrapChoice {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			WrapChoice::None => write!(f, "None"),
			WrapChoice::Word => write!(f, "Word"),
			WrapChoice::Glyph => write!(f, "Glyph"),
			WrapChoice::WordOrGlyph => write!(f, "Word or glyph"),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RenderMode {
	CanvasOnly,
	OutlinesOnly,
	CanvasAndOutlines,
}

impl RenderMode {
	pub(crate) const ALL: [RenderMode; 3] = [Self::CanvasOnly, Self::OutlinesOnly, Self::CanvasAndOutlines];

	pub(crate) fn draw_canvas_text(self) -> bool {
		matches!(self, RenderMode::CanvasOnly | RenderMode::CanvasAndOutlines)
	}

	pub(crate) fn draw_outlines(self) -> bool {
		matches!(self, RenderMode::OutlinesOnly | RenderMode::CanvasAndOutlines)
	}
}

impl Display for RenderMode {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			RenderMode::CanvasOnly => write!(f, "Text"),
			RenderMode::OutlinesOnly => write!(f, "Outlines"),
			RenderMode::CanvasAndOutlines => write!(f, "Both"),
		}
	}
}
