pub use glorp_api::{CanvasTarget, FontChoice, ShapingChoice, WrapChoice};
use iced::Font;

pub trait FontChoiceExt {
	fn to_iced_font(self) -> Font;
}

impl FontChoiceExt for FontChoice {
	fn to_iced_font(self) -> Font {
		match self {
			FontChoice::JetBrainsMono => Font::new("JetBrains Mono"),
			FontChoice::Monospace => Font::MONOSPACE,
			FontChoice::NotoSansCjk => Font::new("Noto Sans CJK SC"),
			FontChoice::SansSerif => Font::DEFAULT,
		}
	}
}

pub trait ShapingChoiceExt {
	fn to_cosmic(self, text: &str) -> cosmic_text::Shaping;
}

impl ShapingChoiceExt for ShapingChoice {
	fn to_cosmic(self, text: &str) -> cosmic_text::Shaping {
		match self {
			ShapingChoice::Auto if text.is_ascii() => cosmic_text::Shaping::Basic,
			ShapingChoice::Basic => cosmic_text::Shaping::Basic,
			ShapingChoice::Auto | ShapingChoice::Advanced => cosmic_text::Shaping::Advanced,
		}
	}
}

pub trait WrapChoiceExt {
	fn to_cosmic(self) -> cosmic_text::Wrap;
}

impl WrapChoiceExt for WrapChoice {
	fn to_cosmic(self) -> cosmic_text::Wrap {
		match self {
			WrapChoice::None => cosmic_text::Wrap::None,
			WrapChoice::Word => cosmic_text::Wrap::Word,
			WrapChoice::Glyph => cosmic_text::Wrap::Glyph,
			WrapChoice::WordOrGlyph => cosmic_text::Wrap::WordOrGlyph,
		}
	}
}
