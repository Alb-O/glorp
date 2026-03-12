use cosmic_text::FontSystem;
use iced::Point;

use super::{ApplyResult, EditorBuffer, EditorCommand};

pub(super) fn apply_command(
	editor: &mut EditorBuffer, font_system: &mut FontSystem, command: EditorCommand,
) -> ApplyResult {
	match command {
		EditorCommand::BeginPointerSelection { position, select_word } => {
			apply_pointer_press(editor, position, select_word)
		}
		EditorCommand::DragPointerSelection(position) => apply_pointer_drag(editor, position),
		EditorCommand::EndPointerSelection => apply_pointer_release(editor),
		EditorCommand::MoveLeft => apply_motion(editor, |editor, layout| editor.move_left(layout)),
		EditorCommand::MoveRight => apply_motion(editor, |editor, layout| editor.move_right(layout)),
		EditorCommand::MoveUp => apply_motion(editor, |editor, layout| editor.move_vertical(layout, -1)),
		EditorCommand::MoveDown => apply_motion(editor, |editor, layout| editor.move_vertical(layout, 1)),
		EditorCommand::MoveLineStart => apply_motion(editor, |editor, layout| editor.move_line_edge(layout, true)),
		EditorCommand::MoveLineEnd => apply_motion(editor, |editor, layout| editor.move_line_edge(layout, false)),
		EditorCommand::EnterInsertBefore => apply_enter_insert(editor, true),
		EditorCommand::EnterInsertAfter => apply_enter_insert(editor, false),
		EditorCommand::ExitInsert => apply_exit_insert(editor),
		EditorCommand::Undo => editor.undo(font_system),
		EditorCommand::Redo => editor.redo(font_system),
		EditorCommand::Backspace => editor.backspace(font_system),
		EditorCommand::DeleteForward => editor.delete_forward(font_system),
		EditorCommand::DeleteSelection => editor.delete_selection(font_system),
		EditorCommand::InsertText(text) => editor.insert_text(font_system, text),
	}
}

fn apply_pointer_press(editor: &mut EditorBuffer, position: Point, select_word: bool) -> ApplyResult {
	let layout = editor.layout_snapshot();
	if select_word {
		editor.select_word_at(&layout, position);
	} else if let Some(cluster_index) = editor.pointer_cluster_index(&layout, position) {
		editor.set_pointer_anchor(layout.cluster(cluster_index).map(|cluster| cluster.byte_range.start));
		editor.select_cluster(&layout, cluster_index);
	} else if editor.document.is_empty() {
		editor.enter_insert_at(0);
		editor.set_selection(None);
	}

	ApplyResult { text_edit: None }
}

fn apply_pointer_drag(editor: &mut EditorBuffer, position: Point) -> ApplyResult {
	let layout = editor.layout_snapshot();
	editor.extend_pointer_selection(&layout, position);
	ApplyResult { text_edit: None }
}

fn apply_pointer_release(editor: &mut EditorBuffer) -> ApplyResult {
	editor.clear_pointer_anchor();
	ApplyResult { text_edit: None }
}

fn apply_motion(
	editor: &mut EditorBuffer, motion: impl FnOnce(&mut EditorBuffer, &super::BufferLayoutSnapshot),
) -> ApplyResult {
	let layout = editor.layout_snapshot();
	motion(editor, &layout);
	ApplyResult { text_edit: None }
}

fn apply_enter_insert(editor: &mut EditorBuffer, before: bool) -> ApplyResult {
	let caret = editor
		.selection()
		.map(|selection| if before { selection.start } else { selection.end })
		.unwrap_or_else(|| if before { 0 } else { editor.document.len() });
	editor.enter_insert_at(caret);
	ApplyResult { text_edit: None }
}

fn apply_exit_insert(editor: &mut EditorBuffer) -> ApplyResult {
	editor.exit_insert();
	ApplyResult { text_edit: None }
}
