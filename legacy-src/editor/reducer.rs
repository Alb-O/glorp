use {
	super::{
		ApplyResult, EditorEditIntent, EditorEngine, EditorHistoryIntent, EditorIntent, EditorModeIntent, EditorMotion,
		EditorPointerIntent,
	},
	crate::scene::DocumentLayout,
	cosmic_text::FontSystem,
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
		EditorPointerIntent::Begin { position, select_word } => apply_pointer_press(editor, position, select_word),
		EditorPointerIntent::Drag(position) => apply_pointer_drag(editor, position),
		EditorPointerIntent::End => apply_pointer_release(editor),
	}
}

fn apply_motion_intent(editor: &mut EditorEngine, intent: EditorMotion) -> ApplyResult {
	match intent {
		EditorMotion::Left => apply_motion(editor, EditorEngine::move_left),
		EditorMotion::Right => apply_motion(editor, EditorEngine::move_right),
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
	let layout = editor.document_layout();
	if select_word {
		editor.select_word_at(&layout, position);
		return layout_result(layout);
	}

	if let Some(cluster_index) = editor.pointer_cluster_index(&layout, position) {
		editor.set_pointer_anchor(layout.cluster(cluster_index).map(|cluster| cluster.byte_range.start));
		editor.select_cluster(&layout, cluster_index);
	} else if editor.core.document.is_empty() {
		// An empty document treats a canvas press as an invitation to place the
		// caret; non-empty blank space stays inert instead of clamping to text.
		editor.enter_insert_at(0);
		editor.set_selection(None);
	}

	layout_result(layout)
}

fn apply_pointer_drag(editor: &mut EditorEngine, position: iced::Point) -> ApplyResult {
	let layout = editor.document_layout();
	editor.extend_pointer_selection(&layout, position);
	layout_result(layout)
}

fn apply_pointer_release(editor: &mut EditorEngine) -> ApplyResult {
	editor.clear_pointer_anchor();
	ApplyResult::default()
}

fn apply_motion(editor: &mut EditorEngine, motion: impl FnOnce(&mut EditorEngine, &DocumentLayout)) -> ApplyResult {
	let layout = editor.document_layout();
	motion(editor, &layout);
	layout_result(layout)
}

fn apply_enter_insert(editor: &mut EditorEngine, before: bool) -> ApplyResult {
	let layout = editor.document_layout();
	let default_caret = if before { 0 } else { editor.core.document.len() };
	let caret = editor.selection().map_or(default_caret, |selection| {
		let range = selection.range();
		if before { range.start } else { range.end }
	});
	editor.set_insert_head(&layout, caret);
	layout_result(layout)
}

fn layout_result(layout: DocumentLayout) -> ApplyResult {
	ApplyResult {
		text_edit: None,
		layout: Some(layout),
		view_refreshed: false,
	}
}

fn apply_exit_insert(editor: &mut EditorEngine) -> ApplyResult {
	editor.exit_insert()
}
