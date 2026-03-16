pub(crate) fn debug_snippet(text: &str) -> String {
	let escaped: String = text.chars().flat_map(char::escape_default).collect();

	if escaped.is_empty() {
		"<empty>".to_string()
	} else {
		format!("\"{escaped}\"")
	}
}

pub(crate) fn line_byte_offsets(text: &str) -> Vec<usize> {
	let mut offsets = vec![0];

	for (index, ch) in text.char_indices() {
		if ch == '\n' {
			offsets.push(index + ch.len_utf8());
		}
	}

	offsets
}
