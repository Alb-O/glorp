use {crate::scene::line_byte_offsets, cosmic_text::Cursor};

pub fn clamp_char_boundary(text: &str, byte: usize) -> usize {
	let mut boundary = byte.min(text.len());
	while boundary > 0 && !text.is_char_boundary(boundary) {
		boundary -= 1;
	}
	boundary
}

pub fn previous_char_boundary(text: &str, byte: usize) -> Option<usize> {
	let byte = clamp_char_boundary(text, byte);
	text[..byte].char_indices().last().map(|(index, _)| index)
}

pub fn next_char_boundary(text: &str, byte: usize) -> Option<usize> {
	let byte = clamp_char_boundary(text, byte);
	text[byte..]
		.char_indices()
		.nth(1)
		.map(|(offset, _)| byte + offset)
		.or_else(|| (byte < text.len()).then_some(text.len()))
}

pub fn previous_char(text: &str, byte: usize) -> Option<(usize, char)> {
	let byte = clamp_char_boundary(text, byte);
	text[..byte].char_indices().last()
}

pub fn next_char(text: &str, byte: usize) -> Option<(usize, char)> {
	let byte = clamp_char_boundary(text, byte);
	text[byte..].chars().next().map(|ch| (byte + ch.len_utf8(), ch))
}

pub fn is_word_char(ch: char) -> bool {
	ch.is_alphanumeric() || ch == '_'
}

pub fn byte_to_cursor(text: &str, byte: usize) -> Cursor {
	let clamped = clamp_char_boundary(text, byte);
	let line_offsets = line_byte_offsets(text);
	let line = line_offsets
		.partition_point(|offset| *offset <= clamped)
		.saturating_sub(1);
	let index = clamped - line_offsets[line];
	Cursor::new(line, index)
}
