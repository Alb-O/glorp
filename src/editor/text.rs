use cosmic_text::Cursor;

pub(super) fn clamp_char_boundary(text: &str, byte: usize) -> usize {
	if byte >= text.len() {
		return text.len();
	}

	let mut boundary = byte;
	while boundary > 0 && !text.is_char_boundary(boundary) {
		boundary -= 1;
	}
	boundary
}

pub(super) fn previous_char_boundary(text: &str, byte: usize) -> Option<usize> {
	let byte = clamp_char_boundary(text, byte);
	text[..byte].char_indices().last().map(|(index, _)| index)
}

pub(super) fn next_char_boundary(text: &str, byte: usize) -> Option<usize> {
	let byte = clamp_char_boundary(text, byte);
	text[byte..]
		.char_indices()
		.nth(1)
		.map(|(offset, _)| byte + offset)
		.or_else(|| (byte < text.len()).then_some(text.len()))
}

pub(super) fn previous_char(text: &str, byte: usize) -> Option<(usize, char)> {
	let byte = clamp_char_boundary(text, byte);
	text[..byte].char_indices().last()
}

pub(super) fn next_char(text: &str, byte: usize) -> Option<(usize, char)> {
	let byte = clamp_char_boundary(text, byte);
	let (_, ch) = text[byte..].char_indices().next()?;
	Some((byte + ch.len_utf8(), ch))
}

pub(super) fn is_word_char(ch: char) -> bool {
	ch.is_alphanumeric() || ch == '_'
}

pub(super) fn byte_to_cursor(text: &str, byte: usize) -> Cursor {
	let (line, index) = byte_to_line_column(text, byte);
	Cursor::new(line, index)
}

pub(super) fn byte_to_line_column(text: &str, byte: usize) -> (usize, usize) {
	let mut clamped = byte.min(text.len());
	while clamped > 0 && !text.is_char_boundary(clamped) {
		clamped -= 1;
	}

	let line_offsets = line_byte_offsets(text);
	let line = line_offsets
		.partition_point(|offset| *offset <= clamped)
		.saturating_sub(1);
	(line, clamped - line_offsets[line])
}

pub(super) fn line_byte_offsets(text: &str) -> Vec<usize> {
	let mut offsets = vec![0];
	for (index, ch) in text.char_indices() {
		if ch == '\n' {
			offsets.push(index + ch.len_utf8());
		}
	}

	offsets
}
