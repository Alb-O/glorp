use iced::Font;
use iced::Point;
use iced::advanced::text::Shaping;
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
	LayoutWidthChanged(f32),
	ShowBaselinesChanged(bool),
	ShowHitboxesChanged(bool),
	SelectSidebarTab(SidebarTab),
	CanvasHovered(Option<CanvasTarget>),
	CanvasClicked {
		target: Option<CanvasTarget>,
		position: Point,
	},
	PaneResized(pane_grid::ResizeEvent),
	EditorCommand(EditorCommand),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SidebarTab {
	Controls,
	Inspect,
	Dump,
}

impl SidebarTab {
	pub(crate) const ALL: [Self; 3] = [Self::Controls, Self::Inspect, Self::Dump];
}

impl Display for SidebarTab {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let label = match self {
			Self::Controls => "Controls",
			Self::Inspect => "Inspect",
			Self::Dump => "Dump",
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
	Mixed,
	Rust,
	Ligatures,
	Arabic,
	Cjk,
	Emoji,
	Custom,
}

impl SamplePreset {
	pub(crate) const ALL: [SamplePreset; 7] = [
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
			RenderMode::CanvasOnly => write!(f, "canvas::Text"),
			RenderMode::OutlinesOnly => write!(f, "Outlines"),
			RenderMode::CanvasAndOutlines => write!(f, "Both"),
		}
	}
}
