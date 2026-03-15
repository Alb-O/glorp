use {
	super::{
		EditorEditIntent, EditorEngine, EditorHistoryIntent, EditorIntent, EditorMode, EditorModeIntent, EditorMotion,
		EditorPointerIntent, TextEdit, geometry::selection_rectangles,
	},
	crate::{
		overlay::{EditorOverlayTone, LayoutRect, OverlayPrimitive, OverlayRectKind},
		scene::{LayoutScene, make_font_system, scene_config},
		types::{FontChoice, RenderMode, ShapingChoice, WrapChoice},
	},
	iced::Point,
};

fn editor(text: &str) -> (cosmic_text::FontSystem, EditorEngine) {
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
	let editor = EditorEngine::new(&mut font_system, text, config);
	(font_system, editor)
}

fn motion(intent: EditorMotion) -> EditorIntent {
	EditorIntent::Motion(intent)
}

fn mode(intent: EditorModeIntent) -> EditorIntent {
	EditorIntent::Mode(intent)
}

fn edit(intent: EditorEditIntent) -> EditorIntent {
	EditorIntent::Edit(intent)
}

fn history(intent: EditorHistoryIntent) -> EditorIntent {
	EditorIntent::History(intent)
}

fn pointer(intent: EditorPointerIntent) -> EditorIntent {
	EditorIntent::Pointer(intent)
}

fn rects(view: &super::EditorViewState, kind: OverlayRectKind) -> Vec<LayoutRect> {
	view.overlays
		.iter()
		.filter_map(|primitive| match primitive {
			OverlayPrimitive::Rect {
				rect,
				kind: primitive_kind,
				..
			} if *primitive_kind == kind => Some(*rect),
			_ => None,
		})
		.collect()
}

fn first_rect(view: &super::EditorViewState, kind: OverlayRectKind) -> LayoutRect {
	rects(view, kind)
		.into_iter()
		.next()
		.expect("expected overlay rectangle")
}

#[test]
fn normal_mode_moves_by_visual_cluster() {
	let (mut font_system, mut editor) = editor("ab\nd");

	assert_eq!(editor.view_state().selection, Some(0..1));

	editor.apply(&mut font_system, motion(EditorMotion::Right));
	assert_eq!(editor.view_state().selection, Some(1..2));

	editor.apply(&mut font_system, motion(EditorMotion::Down));
	assert_eq!(editor.view_state().selection, Some(3..4));
}

#[test]
fn insert_mode_backspace_keeps_caret_on_char_boundaries() {
	let (mut font_system, mut editor) = editor("aé");

	editor.apply(&mut font_system, mode(EditorModeIntent::EnterInsertAfter));
	assert_eq!(editor.view_state().mode, EditorMode::Insert);
	assert_eq!(editor.view_state().selection, Some("a".len().."aé".len()));

	editor.apply(&mut font_system, edit(EditorEditIntent::Backspace));
	assert_eq!(editor.text(), "é");
	assert_eq!(editor.buffer_text(), "é");
	assert_eq!(editor.view_state().selection_head, Some(0));
	assert_eq!(editor.view_state().selection, Some(0.."é".len()));
}

#[test]
fn escape_from_insert_returns_to_normal_selection() {
	let (mut font_system, mut editor) = editor("abc");

	editor.apply(&mut font_system, mode(EditorModeIntent::EnterInsertAfter));
	editor.apply(&mut font_system, motion(EditorMotion::Right));
	editor.apply(&mut font_system, mode(EditorModeIntent::ExitInsert));

	assert_eq!(editor.view_state().mode, EditorMode::Normal);
	assert_eq!(editor.view_state().selection_head, Some(2));
	assert_eq!(editor.view_state().selection, Some(2..3));
}

#[test]
fn undo_and_redo_restore_text_and_caret() {
	let (mut font_system, mut editor) = editor("abc");

	editor.apply(&mut font_system, mode(EditorModeIntent::EnterInsertAfter));
	editor.apply(&mut font_system, edit(EditorEditIntent::InsertText("!".to_string())));

	assert_eq!(editor.text(), "a!bc");
	assert_eq!(editor.view_state().selection_head, Some(2));
	assert_eq!(editor.view_state().selection, Some(2..3));

	editor.apply(&mut font_system, history(EditorHistoryIntent::Undo));
	assert_eq!(editor.text(), "abc");
	assert_eq!(editor.buffer_text(), "abc");
	assert_eq!(editor.view_state().selection_head, Some(1));
	assert_eq!(editor.view_state().selection, Some(1..2));

	editor.apply(&mut font_system, history(EditorHistoryIntent::Redo));
	assert_eq!(editor.text(), "a!bc");
	assert_eq!(editor.buffer_text(), "a!bc");
	assert_eq!(editor.view_state().selection_head, Some(2));
	assert_eq!(editor.view_state().selection, Some(2..3));
}

#[test]
fn enter_insert_and_escape_preserve_visible_selection() {
	let (mut font_system, mut editor) = editor("abc");

	editor.apply(&mut font_system, motion(EditorMotion::Right));
	assert_eq!(editor.view_state().selection, Some(1..2));

	editor.apply(&mut font_system, mode(EditorModeIntent::EnterInsertBefore));
	assert_eq!(editor.view_state().mode, EditorMode::Insert);
	assert_eq!(editor.view_state().selection_head, Some(1));
	assert_eq!(editor.view_state().selection, Some(1..2));

	editor.apply(&mut font_system, mode(EditorModeIntent::ExitInsert));
	assert_eq!(editor.view_state().mode, EditorMode::Normal);
	assert_eq!(editor.view_state().selection_head, Some(1));
	assert_eq!(editor.view_state().selection, Some(1..2));

	editor.apply(&mut font_system, mode(EditorModeIntent::ExitInsert));

	assert_eq!(editor.view_state().mode, EditorMode::Normal);
	assert_eq!(editor.view_state().selection, Some(1..2));
	assert_eq!(editor.view_state().selection_head, Some(1));
}

#[test]
fn reset_rebuilds_document_session_and_layout_together() {
	let (mut font_system, mut editor) = editor("abc");
	let config = scene_config(
		FontChoice::SansSerif,
		ShapingChoice::Advanced,
		WrapChoice::Word,
		RenderMode::CanvasOnly,
		24.0,
		32.0,
		400.0,
	);

	let _ = editor.apply(&mut font_system, mode(EditorModeIntent::EnterInsertAfter));
	let _ = editor.apply(&mut font_system, edit(EditorEditIntent::InsertText("!".to_string())));
	let _ = editor.apply(&mut font_system, motion(EditorMotion::Right));

	assert_eq!(editor.history_depths(), (1, 0));

	editor.reset(&mut font_system, "éz", config);

	assert_eq!(editor.text(), "éz");
	assert_eq!(editor.buffer_text(), "éz");
	assert_eq!(editor.mode(), EditorMode::Normal);
	assert_eq!(editor.history_depths(), (0, 0));
	assert_eq!(editor.view_state().selection, Some(0.."é".len()));
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
	let mut editor = EditorEngine::new(&mut font_system, text, config);

	assert_eq!(
		editor
			.view_state()
			.selection
			.as_ref()
			.and_then(|selection| scene.text.get(selection.clone())),
		Some("🙂")
	);

	editor.apply(&mut font_system, motion(EditorMotion::Down));
	assert_eq!(
		editor
			.view_state()
			.selection
			.as_ref()
			.and_then(|selection| scene.text.get(selection.clone())),
		Some("é")
	);

	assert!(
		editor
			.apply(&mut font_system, edit(EditorEditIntent::DeleteSelection))
			.document_changed
	);
	assert_eq!(editor.text(), "🙂\n");
	assert_eq!(editor.buffer_text(), "🙂\n");
}

#[test]
fn deleting_a_full_line_rebuilds_without_a_zombie_visual_row() {
	let text = "alpha\nbeta\ngamma";
	let config = scene_config(
		FontChoice::SansSerif,
		ShapingChoice::Advanced,
		WrapChoice::None,
		RenderMode::CanvasOnly,
		24.0,
		32.0,
		400.0,
	);
	let mut font_system = make_font_system();
	let mut editor = EditorEngine::new(&mut font_system, text, config);

	editor.apply_document_edit(
		&mut font_system,
		&TextEdit {
			range: 6..11,
			inserted: String::new(),
		},
	);

	assert_eq!(editor.text(), "alpha\ngamma");
	assert_eq!(editor.buffer_text(), "alpha\ngamma");
	assert_eq!(editor.buffer().layout_runs().count(), 2);
}

#[test]
fn motion_intents_report_view_without_document_change() {
	let (mut font_system, mut editor) = editor("abc");

	let outcome = editor.apply(&mut font_system, motion(EditorMotion::Right));

	assert!(!outcome.document_changed);
	assert!(outcome.view_changed);
	assert!(outcome.selection_changed);
	assert!(!outcome.mode_changed);
	assert!(!outcome.requires_scene_rebuild);
	assert_eq!(outcome.text_edit, None);
}

#[test]
fn text_edit_intents_report_document_and_view_outcome() {
	let (mut font_system, mut editor) = editor("abc");

	let _ = editor.apply(&mut font_system, mode(EditorModeIntent::EnterInsertAfter));
	let outcome = editor.apply(&mut font_system, edit(EditorEditIntent::InsertText("!".to_string())));

	assert!(outcome.document_changed);
	assert!(outcome.view_changed);
	assert!(outcome.selection_changed);
	assert!(!outcome.mode_changed);
	assert!(outcome.requires_scene_rebuild);
	assert_eq!(
		outcome.text_edit,
		Some(TextEdit {
			range: 1..1,
			inserted: "!".to_string(),
		})
	);
}

#[test]
fn mode_transition_sets_mode_changed_and_viewport_target() {
	let (mut font_system, mut editor) = editor("abc");

	let outcome = editor.apply(&mut font_system, mode(EditorModeIntent::EnterInsertAfter));

	assert!(outcome.mode_changed);
	assert!(outcome.view_changed);
	assert!(!outcome.document_changed);
	assert!(outcome.viewport_target.is_some());
}

#[test]
fn insert_mode_exposes_a_caret_rectangle() {
	let (mut font_system, mut editor) = editor("abc");

	editor.apply(&mut font_system, mode(EditorModeIntent::EnterInsertAfter));

	let view = editor.view_state();
	let caret = first_rect(&view, OverlayRectKind::EditorCaret(EditorOverlayTone::Insert));
	let active = view
		.viewport_target
		.expect("insert mode should expose an active cluster");

	assert_eq!(caret.y, active.y);
	assert!(caret.height >= active.height);
	assert_eq!(caret.x, active.x);
}

#[test]
fn insert_mode_caret_moves_to_the_trailing_edge_at_line_end() {
	let (mut font_system, mut editor) = editor("abc");

	editor.apply(&mut font_system, motion(EditorMotion::LineEnd));
	editor.apply(&mut font_system, mode(EditorModeIntent::EnterInsertAfter));

	let view = editor.view_state();
	let caret = first_rect(&view, OverlayRectKind::EditorCaret(EditorOverlayTone::Insert));
	let block = view
		.viewport_target
		.expect("line-end insert mode should expose a caret block");
	let layout = editor.layout_snapshot();
	let active = layout
		.cluster(
			layout
				.cluster_at_insert_head(3)
				.expect("line-end insert should resolve a cluster"),
		)
		.expect("line-end insert should retain the last cluster");

	assert!(rects(&view, OverlayRectKind::EditorSelection(EditorOverlayTone::Insert)).is_empty());
	assert!(block.width > caret.width);
	assert!(caret.x > active.x);
	assert!(caret.x >= active.x + active.width - caret.width);
}

#[test]
fn insert_mode_caret_stays_on_the_previous_row_before_a_newline() {
	let (mut font_system, mut editor) = editor("ab\ncd");

	editor.apply(&mut font_system, mode(EditorModeIntent::EnterInsertAfter));
	editor.apply(&mut font_system, motion(EditorMotion::Right));

	let view = editor.view_state();
	let caret = first_rect(&view, OverlayRectKind::EditorCaret(EditorOverlayTone::Insert));
	let active = view
		.viewport_target
		.expect("newline-boundary insert mode should expose a caret target");
	let layout = editor.layout_snapshot();
	let previous = layout
		.cluster(
			layout
				.cluster_at_insert_head(2)
				.expect("newline boundary should resolve a prior cluster"),
		)
		.expect("newline boundary should use the previous row cluster");

	assert_eq!(view.selection_head, Some(2));
	assert_eq!(view.selection, Some(1..2));
	assert_eq!(active.x, caret.x);
	assert_eq!(active.y, caret.y);
	assert!(active.width > caret.width);
	assert!(rects(&view, OverlayRectKind::EditorSelection(EditorOverlayTone::Insert)).is_empty());
	assert_eq!(caret.y, previous.y);
	assert!(caret.x > previous.x);
	assert!(caret.x >= previous.x + previous.width - caret.width);
}

#[test]
fn pointer_miss_does_not_jump_to_first_cluster() {
	let (mut font_system, mut editor) = editor("alpha");

	editor.apply(&mut font_system, motion(EditorMotion::Right));
	assert_eq!(editor.view_state().selection, Some(1..2));

	editor.apply(
		&mut font_system,
		pointer(EditorPointerIntent::BeginSelection {
			position: Point::new(900.0, 900.0),
			select_word: false,
		}),
	);

	assert_eq!(editor.view_state().selection, Some(1..2));
}

#[test]
fn live_selection_rectangles_track_wrapped_width_changes() {
	let text = "alpha beta gamma delta epsilon zeta eta theta";
	let (mut font_system, mut editor) = editor(text);

	for _ in 0..14 {
		editor.apply(&mut font_system, motion(EditorMotion::Right));
	}

	let before = first_rect(
		&editor.view_state(),
		OverlayRectKind::EditorSelection(EditorOverlayTone::Normal),
	);

	editor.sync_buffer_width(&mut font_system, 110.0);

	let after = first_rect(
		&editor.view_state(),
		OverlayRectKind::EditorSelection(EditorOverlayTone::Normal),
	);

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

	let start = first_rect(
		&editor.view_state(),
		OverlayRectKind::EditorSelection(EditorOverlayTone::Normal),
	);

	editor.apply(
		&mut font_system,
		pointer(EditorPointerIntent::BeginSelection {
			position: Point::new(start.x + 2.0, start.y + 2.0),
			select_word: false,
		}),
	);
	editor.apply(
		&mut font_system,
		pointer(EditorPointerIntent::DragSelection(Point::new(90.0, 120.0))),
	);
	editor.apply(&mut font_system, pointer(EditorPointerIntent::EndSelection));

	let view = editor.view_state();
	assert!(rects(&view, OverlayRectKind::EditorSelection(EditorOverlayTone::Normal)).len() >= 2);
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
	let rectangles = selection_rectangles(&layout, &range);

	assert!(!rectangles.is_empty());
	assert!(rectangles.iter().all(|rect| rect.width > 0.0));
}

#[test]
fn double_click_selects_a_full_word() {
	let (mut font_system, mut editor) = editor("alpha beta gamma");

	for _ in 0..11 {
		editor.apply(&mut font_system, motion(EditorMotion::Right));
	}

	let rect = first_rect(
		&editor.view_state(),
		OverlayRectKind::EditorSelection(EditorOverlayTone::Normal),
	);

	editor.apply(
		&mut font_system,
		pointer(EditorPointerIntent::BeginSelection {
			position: Point::new(rect.x + 2.0, rect.y + 2.0),
			select_word: true,
		}),
	);

	let selection = editor
		.view_state()
		.selection
		.expect("double click should produce a selection");
	assert_eq!(editor.text().get(selection), Some("gamma"));
}
