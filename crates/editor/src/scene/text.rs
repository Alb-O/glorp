use std::ops::Range;

pub fn debug_snippet(text: &str) -> String {
	let escaped: String = text.chars().flat_map(char::escape_default).collect();

	if escaped.is_empty() {
		String::from("<empty>")
	} else {
		format!("\"{escaped}\"")
	}
}

pub fn debug_range(text: &str, range: &Range<usize>) -> String {
	text.get(range.clone())
		.map_or_else(|| String::from("<invalid utf8 slice>"), debug_snippet)
}

pub fn line_byte_offsets(text: &str) -> Vec<usize> {
	std::iter::once(0)
		.chain(
			text.char_indices()
				.filter_map(|(index, ch)| (ch == '\n').then_some(index + ch.len_utf8())),
		)
		.collect()
}
