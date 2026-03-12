use super::{EditorBuffer, EditorCommand, EditorMode};
use crate::scene::{LayoutScene, make_font_system, scene_config};
use crate::types::{FontChoice, RenderMode, ShapingChoice, WrapChoice};
use iced::Point;

fn editor(text: &str) -> (cosmic_text::FontSystem, EditorBuffer) {
	let mut font_system = make_font_system();
	let config = scene_config(
		FontChoice::SansSerif,
		ShapingChoice::Advanced,
		WrapChoice::Word,
		RenderMode::CanvasOnly,
		24.0,
		32.0,
		400.0,
	);
	let editor = EditorBuffer::new(&mut font_system, text, config);
	(font_system, editor)
}

#[test]
fn normal_mode_moves_by_visual_cluster() {
	let (mut font_system, mut editor) = editor("ab\nd");

	assert_eq!(editor.view_state().selection, Some(0..1));

	editor.apply(&mut font_system, EditorCommand::MoveRight);
	assert_eq!(editor.view_state().selection, Some(1..2));

	editor.apply(&mut font_system, EditorCommand::MoveDown);
	assert_eq!(editor.view_state().selection, Some(3..4));
}

#[test]
fn insert_mode_backspace_keeps_caret_on_char_boundaries() {
	let (mut font_system, mut editor) = editor("aé");

	editor.apply(&mut font_system, EditorCommand::EnterInsertAfter);
	assert_eq!(editor.view_state().mode, EditorMode::Insert);

	editor.apply(&mut font_system, EditorCommand::Backspace);
	assert_eq!(editor.text(), "é");
	assert_eq!(editor.buffer_text(), "é");
	assert_eq!(editor.view_state().caret, 0);
}

#[test]
fn escape_from_insert_returns_to_normal_selection() {
	let (mut font_system, mut editor) = editor("abc");

	editor.apply(&mut font_system, EditorCommand::EnterInsertAfter);
	editor.apply(&mut font_system, EditorCommand::MoveRight);
	editor.apply(&mut font_system, EditorCommand::ExitInsert);

	assert_eq!(editor.view_state().mode, EditorMode::Normal);
	assert_eq!(editor.view_state().selection, Some(1..2));
}

#[test]
fn undo_and_redo_restore_text_and_caret() {
	let (mut font_system, mut editor) = editor("abc");

	editor.apply(&mut font_system, EditorCommand::EnterInsertAfter);
	editor.apply(&mut font_system, EditorCommand::InsertText("!".to_string()));

	assert_eq!(editor.text(), "a!bc");
	assert_eq!(editor.view_state().caret, 2);

	editor.apply(&mut font_system, EditorCommand::Undo);
	assert_eq!(editor.text(), "abc");
	assert_eq!(editor.buffer_text(), "abc");
	assert_eq!(editor.view_state().caret, 1);

	editor.apply(&mut font_system, EditorCommand::Redo);
	assert_eq!(editor.text(), "a!bc");
	assert_eq!(editor.buffer_text(), "a!bc");
	assert_eq!(editor.view_state().caret, 2);
}

#[test]
fn delete_selection_on_later_line_handles_multibyte_text() {
	let text = "🙂\né";
	let mut font_system = make_font_system();
	let scene = LayoutScene::build(
		&mut font_system,
		text.to_string(),
		FontChoice::SansSerif,
		ShapingChoice::Advanced,
		WrapChoice::None,
		24.0,
		32.0,
		400.0,
		RenderMode::CanvasAndOutlines,
	);
	let config = scene_config(
		FontChoice::SansSerif,
		ShapingChoice::Advanced,
		WrapChoice::None,
		RenderMode::CanvasAndOutlines,
		24.0,
		32.0,
		400.0,
	);
	let mut editor = EditorBuffer::new(&mut font_system, text, config);

	assert_eq!(
		editor
			.view_state()
			.selection
			.as_ref()
			.and_then(|selection| scene.text.get(selection.clone())),
		Some("🙂")
	);

	editor.apply(&mut font_system, EditorCommand::MoveDown);
	assert_eq!(
		editor
			.view_state()
			.selection
			.as_ref()
			.and_then(|selection| scene.text.get(selection.clone())),
		Some("é")
	);

	assert!(editor.apply(&mut font_system, EditorCommand::DeleteSelection).changed);
	assert_eq!(editor.text(), "🙂\n");
	assert_eq!(editor.buffer_text(), "🙂\n");
}

#[test]
fn live_selection_rectangles_track_wrapped_width_changes() {
	let text = "alpha beta gamma delta epsilon zeta eta theta";
	let (mut font_system, mut editor) = editor(text);

	for _ in 0..14 {
		editor.apply(&mut font_system, EditorCommand::MoveRight);
	}

	let before = editor
		.view_state()
		.selection_rectangles
		.first()
		.copied()
		.expect("selection geometry should exist before resize");

	editor.sync_buffer_width(&mut font_system, 110.0);

	let after = editor
		.view_state()
		.selection_rectangles
		.first()
		.copied()
		.expect("selection geometry should exist after resize");

	assert!(
		after.y > before.y || (after.y == before.y && after.x < before.x),
		"expected wrapped selection to move after width shrink, before={before:?} after={after:?}"
	);
}

#[test]
fn drag_selection_spans_multiple_wrapped_rectangles() {
	let text = "alpha beta gamma delta epsilon zeta eta theta";
	let (mut font_system, mut editor) = editor(text);
	editor.sync_buffer_width(&mut font_system, 130.0);

	let start = editor
		.view_state()
		.selection_rectangles
		.first()
		.copied()
		.expect("initial selection should have a rectangle");

	editor.apply(
		&mut font_system,
		EditorCommand::BeginPointerSelection {
			position: Point::new(start.x + 2.0, start.y + 2.0),
			select_word: false,
		},
	);
	editor.apply(
		&mut font_system,
		EditorCommand::DragPointerSelection(Point::new(90.0, 120.0)),
	);
	editor.apply(&mut font_system, EditorCommand::EndPointerSelection);

	let view = editor.view_state();
	assert!(view.selection_rectangles.len() >= 2);
	assert!(
		view.selection
			.as_ref()
			.is_some_and(|selection| selection.end > selection.start)
	);
}

#[test]
fn rtl_cluster_byte_lookup_matches_visual_cluster() {
	let text = "السلام عليكم السلام عليكم السلام عليكم";
	let (mut font_system, mut editor) = editor(text);
	editor.sync_buffer_width(&mut font_system, 150.0);

	let layout = editor.layout_snapshot();
	let rtl_indices = layout
		.clusters()
		.iter()
		.enumerate()
		.filter_map(|(index, cluster)| {
			(layout.cluster_at_or_after(cluster.byte_range.start) == Some(index)).then_some(index)
		})
		.collect::<Vec<_>>();

	assert!(
		rtl_indices.len() >= 4,
		"expected byte lookup to round-trip multiple Arabic clusters, got {rtl_indices:?}"
	);

	for (index, cluster) in layout.clusters().iter().enumerate() {
		assert_eq!(
			layout.cluster_at_or_after(cluster.byte_range.start),
			Some(index),
			"cluster byte lookup diverged for cluster {index:?} with range {:?}",
			cluster.byte_range,
		);
	}
}

#[test]
fn rtl_selection_rectangles_keep_positive_width() {
	let text = "السلام عليكم السلام عليكم السلام عليكم";
	let (mut font_system, mut editor) = editor(text);
	editor.sync_buffer_width(&mut font_system, 150.0);

	let layout = editor.layout_snapshot();
	let rtl_span = layout
		.clusters()
		.windows(2)
		.find(|pair| {
			pair[0].run_index == pair[1].run_index
				&& pair[0].byte_range.start < pair[1].byte_range.start
				&& pair[0].x > pair[1].x
		})
		.expect("expected a visual rtl span in the wrapped Arabic sample");

	let range = rtl_span[0].byte_range.start.min(rtl_span[1].byte_range.start)
		..rtl_span[0].byte_range.end.max(rtl_span[1].byte_range.end);
	let rectangles = layout.selection_rectangles(&range);

	assert!(!rectangles.is_empty());
	assert!(rectangles.iter().all(|rect| rect.width > 0.0));
}

#[test]
fn double_click_selects_a_full_word() {
	let (mut font_system, mut editor) = editor("alpha beta gamma");

	for _ in 0..11 {
		editor.apply(&mut font_system, EditorCommand::MoveRight);
	}

	let rect = editor
		.view_state()
		.selection_rectangles
		.first()
		.copied()
		.expect("selection should have a rectangle");

	editor.apply(
		&mut font_system,
		EditorCommand::BeginPointerSelection {
			position: Point::new(rect.x + 2.0, rect.y + 2.0),
			select_word: true,
		},
	);

	let selection = editor
		.view_state()
		.selection
		.expect("double click should produce a selection");
	assert_eq!(editor.text().get(selection), Some("gamma"));
}
