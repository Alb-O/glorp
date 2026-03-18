#[allow(dead_code)]
pub fn enum_values() -> [&'static [&'static str]; 4] {
	[
		&["left", "right", "up", "down", "line-start", "line-end"],
		&["enter-insert-before", "enter-insert-after", "exit-insert"],
		&["undo", "redo"],
		&["controls", "inspect", "perf"],
	]
}
