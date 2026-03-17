use {
	crate::editor::{EditorIntent, EditorPointerIntent},
	iced::{Font, Size, Vector, time::Instant, widget::pane_grid},
	std::{
		fmt::{self, Display},
		hash::Hash,
	},
};

#[derive(Debug, Clone)]
pub(crate) enum Message {
	Controls(ControlsMessage),
	Sidebar(SidebarMessage),
	Canvas(CanvasEvent),
	Editor(EditorIntent),
	Perf(PerfMessage),
	Viewport(ViewportMessage),
	Shell(ShellMessage),
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum ControlsMessage {
	LoadPreset(SamplePreset),
	FontSelected(FontChoice),
	ShapingSelected(ShapingChoice),
	WrappingSelected(WrapChoice),
	FontSizeChanged(f32),
	LineHeightChanged(f32),
	ShowBaselinesChanged(bool),
	ShowHitboxesChanged(bool),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SidebarMessage {
	SelectTab(SidebarTab),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum CanvasEvent {
	Hovered(Option<CanvasTarget>),
	FocusChanged(bool),
	ScrollChanged(Vector),
	PointerSelectionStarted {
		target: Option<CanvasTarget>,
		intent: EditorPointerIntent,
	},
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PerfMessage {
	Tick(Instant),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum ViewportMessage {
	CanvasResized(Size),
	ResizeTick(Instant),
}

#[derive(Debug, Clone)]
pub(crate) enum ShellMessage {
	PaneResized(pane_grid::ResizeEvent),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum SidebarTab {
	Controls,
	Inspect,
	Perf,
}

impl SidebarTab {
	pub(crate) const ALL: [Self; 3] = [Self::Controls, Self::Inspect, Self::Perf];

	pub(crate) const fn label(self) -> &'static str {
		match self {
			Self::Controls => "Controls",
			Self::Inspect => "Inspect",
			Self::Perf => "Perf",
		}
	}
}

impl Display for SidebarTab {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(self.label())
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum CanvasTarget {
	Run(usize),
	Cluster(usize),
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
		f.write_str(match self {
			SamplePreset::Tall => "Tall",
			SamplePreset::Mixed => "Mixed",
			SamplePreset::Rust => "Rust",
			SamplePreset::Ligatures => "Ligatures",
			SamplePreset::Arabic => "Arabic",
			SamplePreset::Cjk => "CJK",
			SamplePreset::Emoji => "Emoji",
			SamplePreset::Custom => "Custom",
		})
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
			FontChoice::JetBrainsMono => Font::new("JetBrains Mono"),
			FontChoice::Monospace => Font::MONOSPACE,
			FontChoice::NotoSansCjk => Font::new("Noto Sans CJK SC"),
			FontChoice::SansSerif => Font::DEFAULT,
		}
	}
}

impl Display for FontChoice {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(match self {
			FontChoice::JetBrainsMono => "JetBrains Mono",
			FontChoice::Monospace => "Monospace family",
			FontChoice::NotoSansCjk => "Noto Sans CJK SC",
			FontChoice::SansSerif => "Sans Serif family",
		})
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

	pub(crate) fn to_cosmic(self, text: &str) -> cosmic_text::Shaping {
		match self {
			ShapingChoice::Auto if text.is_ascii() => cosmic_text::Shaping::Basic,
			ShapingChoice::Auto => cosmic_text::Shaping::Advanced,
			ShapingChoice::Basic => cosmic_text::Shaping::Basic,
			ShapingChoice::Advanced => cosmic_text::Shaping::Advanced,
		}
	}
}

impl Display for ShapingChoice {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(match self {
			ShapingChoice::Auto => "Auto",
			ShapingChoice::Basic => "Basic",
			ShapingChoice::Advanced => "Advanced",
		})
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
		f.write_str(match self {
			WrapChoice::None => "None",
			WrapChoice::Word => "Word",
			WrapChoice::Glyph => "Glyph",
			WrapChoice::WordOrGlyph => "Word or glyph",
		})
	}
}
