pub use glorp_api::{CanvasTarget, FontChoice, ShapingChoice, WrapChoice};
use iced::Font;

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
