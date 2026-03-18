pub use glorp_api::{CanvasTarget, FontChoice, SamplePreset, ShapingChoice, SidebarTab, WrapChoice};
use {
	glorp_editor::{EditorIntent, EditorPointerIntent},
	iced::{Size, Vector, time::Instant, widget::pane_grid},
};

pub const SAMPLE_PRESETS: [SamplePreset; 8] = [
	SamplePreset::Tall,
	SamplePreset::Mixed,
	SamplePreset::Rust,
	SamplePreset::Ligatures,
	SamplePreset::Arabic,
	SamplePreset::Cjk,
	SamplePreset::Emoji,
	SamplePreset::Custom,
];

pub const FONT_CHOICES: [FontChoice; 4] = [
	FontChoice::JetBrainsMono,
	FontChoice::Monospace,
	FontChoice::NotoSansCjk,
	FontChoice::SansSerif,
];

pub const SHAPING_CHOICES: [ShapingChoice; 3] = [ShapingChoice::Auto, ShapingChoice::Basic, ShapingChoice::Advanced];

pub const WRAP_CHOICES: [WrapChoice; 4] = [
	WrapChoice::None,
	WrapChoice::Word,
	WrapChoice::Glyph,
	WrapChoice::WordOrGlyph,
];

pub const SIDEBAR_TABS: [SidebarTab; 3] = [SidebarTab::Controls, SidebarTab::Inspect, SidebarTab::Perf];

#[derive(Debug, Clone)]
pub enum Message {
	Controls(ControlsMessage),
	Sidebar(SidebarMessage),
	Canvas(CanvasEvent),
	Editor(EditorIntent),
	Perf(PerfMessage),
	Viewport(ViewportMessage),
	Shell(ShellMessage),
}

#[derive(Debug, Clone, Copy)]
pub enum ControlsMessage {
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
pub enum SidebarMessage {
	SelectTab(SidebarTab),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CanvasEvent {
	Hovered(Option<CanvasTarget>),
	FocusChanged(bool),
	ScrollChanged(Vector),
	PointerSelectionStarted {
		target: Option<CanvasTarget>,
		intent: EditorPointerIntent,
	},
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerfMessage {
	Tick(Instant),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ViewportMessage {
	CanvasResized(Size),
}

#[derive(Debug, Clone)]
pub enum ShellMessage {
	PaneResized(pane_grid::ResizeEvent),
}

pub const fn sample_preset_label(preset: SamplePreset) -> &'static str {
	match preset {
		SamplePreset::Tall => "Tall",
		SamplePreset::Mixed => "Mixed",
		SamplePreset::Rust => "Rust",
		SamplePreset::Ligatures => "Ligatures",
		SamplePreset::Arabic => "Arabic",
		SamplePreset::Cjk => "CJK",
		SamplePreset::Emoji => "Emoji",
		SamplePreset::Custom => "Custom",
	}
}

pub const fn sample_preset_text(preset: SamplePreset) -> &'static str {
	match preset {
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

pub const fn font_choice_label(font: FontChoice) -> &'static str {
	match font {
		FontChoice::JetBrainsMono => "JetBrains Mono",
		FontChoice::Monospace => "Monospace family",
		FontChoice::NotoSansCjk => "Noto Sans CJK SC",
		FontChoice::SansSerif => "Sans Serif family",
	}
}

pub const fn shaping_choice_label(shaping: ShapingChoice) -> &'static str {
	match shaping {
		ShapingChoice::Auto => "Auto",
		ShapingChoice::Basic => "Basic",
		ShapingChoice::Advanced => "Advanced",
	}
}

pub const fn wrap_choice_label(wrapping: WrapChoice) -> &'static str {
	match wrapping {
		WrapChoice::None => "None",
		WrapChoice::Word => "Word",
		WrapChoice::Glyph => "Glyph",
		WrapChoice::WordOrGlyph => "Word or glyph",
	}
}

pub const fn sidebar_tab_label(tab: SidebarTab) -> &'static str {
	match tab {
		SidebarTab::Controls => "Controls",
		SidebarTab::Inspect => "Inspect",
		SidebarTab::Perf => "Perf",
	}
}
