use cosmic_text::FontSystem;

use super::{
	ApplyResult, EditorEditIntent, EditorEngine, EditorHistoryIntent, EditorIntent, EditorModeIntent, EditorMotion,
	EditorPointerIntent,
};

pub(super) fn apply_intent(
	editor: &mut EditorEngine, font_system: &mut FontSystem, intent: EditorIntent,
) -> ApplyResult {
	match intent {
		EditorIntent::Pointer(intent) => apply_pointer_intent(editor, intent),
		EditorIntent::Motion(intent) => apply_motion_intent(editor, intent),
		EditorIntent::Mode(intent) => apply_mode_intent(editor, intent),
		EditorIntent::Edit(intent) => apply_edit_intent(editor, font_system, intent),
		EditorIntent::History(intent) => apply_history_intent(editor, font_system, intent),
	}
}

fn apply_pointer_intent(editor: &mut EditorEngine, intent: EditorPointerIntent) -> ApplyResult {
	match intent {
		EditorPointerIntent::BeginSelection { position, select_word } => {
			apply_pointer_press(editor, position, select_word)
		}
		EditorPointerIntent::DragSelection(position) => apply_pointer_drag(editor, position),
		EditorPointerIntent::EndSelection => apply_pointer_release(editor),
	}
}

fn apply_motion_intent(editor: &mut EditorEngine, intent: EditorMotion) -> ApplyResult {
	match intent {
		EditorMotion::Left => apply_motion(editor, |editor, layout| editor.move_left(layout)),
		EditorMotion::Right => apply_motion(editor, |editor, layout| editor.move_right(layout)),
		EditorMotion::Up => apply_motion(editor, |editor, layout| editor.move_vertical(layout, -1)),
		EditorMotion::Down => apply_motion(editor, |editor, layout| editor.move_vertical(layout, 1)),
		EditorMotion::LineStart => apply_motion(editor, |editor, layout| editor.move_line_edge(layout, true)),
		EditorMotion::LineEnd => apply_motion(editor, |editor, layout| editor.move_line_edge(layout, false)),
	}
}

fn apply_mode_intent(editor: &mut EditorEngine, intent: EditorModeIntent) -> ApplyResult {
	match intent {
		EditorModeIntent::EnterInsertBefore => apply_enter_insert(editor, true),
		EditorModeIntent::EnterInsertAfter => apply_enter_insert(editor, false),
		EditorModeIntent::ExitInsert => apply_exit_insert(editor),
	}
}

fn apply_edit_intent(editor: &mut EditorEngine, font_system: &mut FontSystem, intent: EditorEditIntent) -> ApplyResult {
	match intent {
		EditorEditIntent::Backspace => editor.backspace(font_system),
		EditorEditIntent::DeleteForward => editor.delete_forward(font_system),
		EditorEditIntent::DeleteSelection => editor.delete_selection(font_system),
		EditorEditIntent::InsertText(text) => editor.insert_text(font_system, text),
	}
}

fn apply_history_intent(
	editor: &mut EditorEngine, font_system: &mut FontSystem, intent: EditorHistoryIntent,
) -> ApplyResult {
	match intent {
		EditorHistoryIntent::Undo => editor.undo(font_system),
		EditorHistoryIntent::Redo => editor.redo(font_system),
	}
}

fn apply_pointer_press(editor: &mut EditorEngine, position: iced::Point, select_word: bool) -> ApplyResult {
	let layout = editor.layout_snapshot();
	if select_word {
		editor.select_word_at(&layout, position);
	} else if let Some(cluster_index) = editor.pointer_cluster_index(&layout, position) {
		editor.set_pointer_anchor(layout.cluster(cluster_index).map(|cluster| cluster.byte_range.start));
		editor.select_cluster(&layout, cluster_index);
	} else if editor.state.document.is_empty() {
		editor.enter_insert_at(0);
		editor.set_selection(None);
	}

	ApplyResult {
		text_edit: None,
		layout: Some(layout),
	}
}

fn apply_pointer_drag(editor: &mut EditorEngine, position: iced::Point) -> ApplyResult {
	let layout = editor.layout_snapshot();
	editor.extend_pointer_selection(&layout, position);
	ApplyResult {
		text_edit: None,
		layout: Some(layout),
	}
}

fn apply_pointer_release(editor: &mut EditorEngine) -> ApplyResult {
	editor.clear_pointer_anchor();
	ApplyResult::default()
}

fn apply_motion(
	editor: &mut EditorEngine, motion: impl FnOnce(&mut EditorEngine, &super::BufferLayoutSnapshot),
) -> ApplyResult {
	let layout = editor.layout_snapshot();
	motion(editor, &layout);
	ApplyResult {
		text_edit: None,
		layout: Some(layout),
	}
}

fn apply_enter_insert(editor: &mut EditorEngine, before: bool) -> ApplyResult {
	let layout = editor.layout_snapshot();
	let caret = editor
		.selection()
		.map(|selection| {
			if before {
				selection.range().start
			} else {
				selection.range().end
			}
		})
		.unwrap_or_else(|| if before { 0 } else { editor.state.document.len() });
	editor.enter_insert_with_layout(&layout, caret);
	ApplyResult {
		text_edit: None,
		layout: Some(layout),
	}
}

fn apply_exit_insert(editor: &mut EditorEngine) -> ApplyResult {
	editor.exit_insert()
}
