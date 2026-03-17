pub(crate) fn debug_snippet(text: &str) -> String {
	let escaped: String = text.chars().flat_map(char::escape_default).collect();

	if escaped.is_empty() {
		"<empty>".to_string()
	} else {
		format!("\"{escaped}\"")
	}
}

pub(crate) fn line_byte_offsets(text: &str) -> Vec<usize> {
	std::iter::once(0)
		.chain(
			text.char_indices()
				.filter_map(|(index, ch)| (ch == '\n').then_some(index + ch.len_utf8())),
		)
		.collect()
}
