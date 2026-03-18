pub use glorp_api::{FontChoice, ShapingChoice, WrapChoice};
use iced::Font;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CanvasTarget {
	Run(usize),
	Cluster(usize),
}

pub const fn sample_preset_text(preset: glorp_api::SamplePreset) -> &'static str {
	match preset {
		glorp_api::SamplePreset::Tall => concat!(
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
		glorp_api::SamplePreset::Mixed => "office affine ffi ffl\n漢字カタカナ and Latin\nالسلام عليكم\nemoji 🙂🚀👩‍💻",
		glorp_api::SamplePreset::Rust => "fn main() {\n    println!(\"ffi -> office -> 汉字\");\n}\n",
		glorp_api::SamplePreset::Ligatures => "office affine final fluff ffi ffl fj",
		glorp_api::SamplePreset::Arabic => "السلام عليكم\nمرحبا بالعالم",
		glorp_api::SamplePreset::Cjk => "漢字かなカナ\n混在テキスト with ASCII",
		glorp_api::SamplePreset::Emoji => "🙂🚀👩‍💻 text + emoji fallback",
		glorp_api::SamplePreset::Custom => "",
	}
}

pub trait FontChoiceExt {
	fn to_iced_font(self) -> Font;
}

impl FontChoiceExt for FontChoice {
	fn to_iced_font(self) -> Font {
		match self {
			Self::JetBrainsMono => Font::new("JetBrains Mono"),
			Self::Monospace => Font::MONOSPACE,
			Self::NotoSansCjk => Font::new("Noto Sans CJK SC"),
			Self::SansSerif => Font::DEFAULT,
		}
	}
}

pub trait ShapingChoiceExt {
	fn to_cosmic(self, text: &str) -> cosmic_text::Shaping;
}

impl ShapingChoiceExt for ShapingChoice {
	fn to_cosmic(self, text: &str) -> cosmic_text::Shaping {
		match self {
			Self::Auto if text.is_ascii() => cosmic_text::Shaping::Basic,
			Self::Basic => cosmic_text::Shaping::Basic,
			Self::Auto | Self::Advanced => cosmic_text::Shaping::Advanced,
		}
	}
}

pub trait WrapChoiceExt {
	fn to_cosmic(self) -> cosmic_text::Wrap;
}

impl WrapChoiceExt for WrapChoice {
	fn to_cosmic(self) -> cosmic_text::Wrap {
		match self {
			Self::None => cosmic_text::Wrap::None,
			Self::Word => cosmic_text::Wrap::Word,
			Self::Glyph => cosmic_text::Wrap::Glyph,
			Self::WordOrGlyph => cosmic_text::Wrap::WordOrGlyph,
		}
	}
}
