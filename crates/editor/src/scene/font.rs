use {
	super::SceneConfig,
	crate::types::{FontChoice, ShapingChoice, ShapingChoiceExt, WrapChoice, WrapChoiceExt},
	cosmic_text::{Attrs, Buffer, FontSystem, Metrics},
	iced::Font,
};

#[must_use]
pub fn make_font_system() -> FontSystem {
	let mut font_system = FontSystem::new();
	let db = font_system.db_mut();
	db.set_monospace_family("JetBrains Mono");
	db.set_sans_serif_family("Noto Sans CJK SC");
	font_system
}

#[must_use]
pub fn scene_config(
	font_choice: FontChoice, shaping: ShapingChoice, wrapping: WrapChoice, font_size: f32, line_height: f32,
	max_width: f32,
) -> SceneConfig {
	SceneConfig {
		font_choice,
		shaping,
		wrapping,
		font_size,
		line_height,
		max_width,
	}
}

pub fn build_buffer(font_system: &mut FontSystem, text: &str, config: SceneConfig) -> Buffer {
	let mut buffer = Buffer::new(font_system, Metrics::new(config.font_size, config.line_height));
	buffer.set_size(font_system, Some(config.max_width), None);
	buffer.set_wrap(font_system, config.wrapping.to_cosmic());
	buffer.set_text(
		font_system,
		text,
		&to_attributes(config.font()),
		config.shaping.to_cosmic(text),
		None,
	);
	buffer
}

fn to_attributes(font: Font) -> Attrs<'static> {
	Attrs::new()
		.family(to_family(font.family))
		.weight(to_weight(font.weight))
		.stretch(to_stretch(font.stretch))
		.style(to_style(font.style))
}

fn to_family(family: iced::font::Family) -> cosmic_text::Family<'static> {
	match family {
		iced::font::Family::Name(name) => cosmic_text::Family::Name(name),
		iced::font::Family::SansSerif => cosmic_text::Family::SansSerif,
		iced::font::Family::Serif => cosmic_text::Family::Serif,
		iced::font::Family::Cursive => cosmic_text::Family::Cursive,
		iced::font::Family::Fantasy => cosmic_text::Family::Fantasy,
		iced::font::Family::Monospace => cosmic_text::Family::Monospace,
	}
}

fn to_weight(weight: iced::font::Weight) -> cosmic_text::Weight {
	match weight {
		iced::font::Weight::Thin => cosmic_text::Weight::THIN,
		iced::font::Weight::ExtraLight => cosmic_text::Weight::EXTRA_LIGHT,
		iced::font::Weight::Light => cosmic_text::Weight::LIGHT,
		iced::font::Weight::Normal => cosmic_text::Weight::NORMAL,
		iced::font::Weight::Medium => cosmic_text::Weight::MEDIUM,
		iced::font::Weight::Semibold => cosmic_text::Weight::SEMIBOLD,
		iced::font::Weight::Bold => cosmic_text::Weight::BOLD,
		iced::font::Weight::ExtraBold => cosmic_text::Weight::EXTRA_BOLD,
		iced::font::Weight::Black => cosmic_text::Weight::BLACK,
	}
}

fn to_stretch(stretch: iced::font::Stretch) -> cosmic_text::Stretch {
	match stretch {
		iced::font::Stretch::UltraCondensed => cosmic_text::Stretch::UltraCondensed,
		iced::font::Stretch::ExtraCondensed => cosmic_text::Stretch::ExtraCondensed,
		iced::font::Stretch::Condensed => cosmic_text::Stretch::Condensed,
		iced::font::Stretch::SemiCondensed => cosmic_text::Stretch::SemiCondensed,
		iced::font::Stretch::Normal => cosmic_text::Stretch::Normal,
		iced::font::Stretch::SemiExpanded => cosmic_text::Stretch::SemiExpanded,
		iced::font::Stretch::Expanded => cosmic_text::Stretch::Expanded,
		iced::font::Stretch::ExtraExpanded => cosmic_text::Stretch::ExtraExpanded,
		iced::font::Stretch::UltraExpanded => cosmic_text::Stretch::UltraExpanded,
	}
}

fn to_style(style: iced::font::Style) -> cosmic_text::Style {
	match style {
		iced::font::Style::Normal => cosmic_text::Style::Normal,
		iced::font::Style::Italic => cosmic_text::Style::Italic,
		iced::font::Style::Oblique => cosmic_text::Style::Oblique,
	}
}
