use std::fmt::{self, Display};
use std::ops::Range;

use iced::Point;

use crate::scene::LayoutScene;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EditorMode {
	Normal,
	Insert,
}

impl Display for EditorMode {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Normal => f.write_str("Normal"),
			Self::Insert => f.write_str("Insert"),
		}
	}
}

#[derive(Debug, Clone)]
pub(crate) struct EditorViewState {
	pub(crate) mode: EditorMode,
	pub(crate) selection: Option<Range<usize>>,
	pub(crate) caret: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct EditorBuffer {
	text: String,
	mode: EditorMode,
	selection: Option<Range<usize>>,
	caret: usize,
	preferred_x: Option<f32>,
}

#[derive(Debug, Clone)]
pub(crate) enum EditorCommand {
	SelectClusterAt(Point),
	MoveLeft,
	MoveRight,
	MoveUp,
	MoveDown,
	MoveLineStart,
	MoveLineEnd,
	EnterInsertBefore,
	EnterInsertAfter,
	ExitInsert,
	Backspace,
	DeleteForward,
	DeleteSelection,
	InsertText(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TextEdit {
	pub(crate) range: Range<usize>,
	pub(crate) inserted: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ApplyResult {
	pub(crate) changed: bool,
	pub(crate) text_edit: Option<TextEdit>,
}

impl EditorBuffer {
	pub(crate) fn new(text: impl Into<String>) -> Self {
		Self {
			text: text.into(),
			mode: EditorMode::Normal,
			selection: None,
			caret: 0,
			preferred_x: None,
		}
	}

	pub(crate) fn text(&self) -> &str {
		&self.text
	}

	pub(crate) fn mode(&self) -> EditorMode {
		self.mode
	}

	pub(crate) fn reset(&mut self, text: impl Into<String>) {
		self.text = text.into();
		self.mode = EditorMode::Normal;
		self.selection = None;
		self.caret = 0;
		self.preferred_x = None;
	}

	pub(crate) fn view_state(&self) -> EditorViewState {
		EditorViewState {
			mode: self.mode,
			selection: self.selection.clone(),
			caret: self.caret,
		}
	}

	pub(crate) fn apply(&mut self, command: EditorCommand, scene: &LayoutScene) -> ApplyResult {
		match command {
			EditorCommand::SelectClusterAt(point) => {
				if let Some(cluster_index) = scene.hit_test_cluster(point) {
					self.select_cluster(scene, cluster_index);
				} else if self.text.is_empty() {
					self.mode = EditorMode::Insert;
					self.caret = 0;
					self.preferred_x = None;
				}
				ApplyResult {
					changed: false,
					text_edit: None,
				}
			}
			EditorCommand::MoveLeft => {
				self.move_left(scene);
				ApplyResult {
					changed: false,
					text_edit: None,
				}
			}
			EditorCommand::MoveRight => {
				self.move_right(scene);
				ApplyResult {
					changed: false,
					text_edit: None,
				}
			}
			EditorCommand::MoveUp => {
				self.move_vertical(scene, -1);
				ApplyResult {
					changed: false,
					text_edit: None,
				}
			}
			EditorCommand::MoveDown => {
				self.move_vertical(scene, 1);
				ApplyResult {
					changed: false,
					text_edit: None,
				}
			}
			EditorCommand::MoveLineStart => {
				self.move_line_edge(scene, true);
				ApplyResult {
					changed: false,
					text_edit: None,
				}
			}
			EditorCommand::MoveLineEnd => {
				self.move_line_edge(scene, false);
				ApplyResult {
					changed: false,
					text_edit: None,
				}
			}
			EditorCommand::EnterInsertBefore => {
				self.mode = EditorMode::Insert;
				self.caret = self
					.current_selection(scene)
					.map(|cluster| cluster.byte_range.start)
					.unwrap_or(0);
				self.preferred_x = None;
				ApplyResult {
					changed: false,
					text_edit: None,
				}
			}
			EditorCommand::EnterInsertAfter => {
				self.mode = EditorMode::Insert;
				self.caret = self
					.current_selection(scene)
					.map(|cluster| cluster.byte_range.end)
					.unwrap_or(self.text.len());
				self.preferred_x = None;
				ApplyResult {
					changed: false,
					text_edit: None,
				}
			}
			EditorCommand::ExitInsert => {
				self.exit_insert(scene);
				ApplyResult {
					changed: false,
					text_edit: None,
				}
			}
			EditorCommand::Backspace => self.backspace(scene),
			EditorCommand::DeleteForward => self.delete_forward(scene),
			EditorCommand::DeleteSelection => self.delete_selection(scene),
			EditorCommand::InsertText(text) => self.insert_text(text),
		}
	}

	pub(crate) fn sync_with_scene(&mut self, scene: &LayoutScene) {
		self.caret = clamp_char_boundary(&self.text, self.caret);

		match self.mode {
			EditorMode::Insert => {}
			EditorMode::Normal => {
				self.selection = if scene.clusters().is_empty() {
					None
				} else if let Some(selection) = self.selection.clone() {
					scene
						.cluster_index_for_range(&selection)
						.or_else(|| scene.cluster_at_or_after(selection.start))
						.or_else(|| scene.cluster_before(selection.start))
						.and_then(|index| scene.cluster(index))
						.map(|cluster| cluster.byte_range.clone())
				} else {
					scene.cluster(0).map(|cluster| cluster.byte_range.clone())
				};

				if let Some(cluster) = self.current_selection(scene) {
					self.preferred_x = Some(cluster.center_x());
				}
			}
		}
	}

	pub(crate) fn selection_details(&self, scene: &LayoutScene) -> String {
		match self.mode {
			EditorMode::Normal => {
				let Some(cluster) = self.current_selection(scene) else {
					return format!("  mode: {}\n  selection: none", self.mode);
				};

				format!(
					"  mode: {}\n  cluster: {}\n  bytes: {:?}\n  run: {}\n  x/y: {:.1}, {:.1}\n  w/h: {:.1}, {:.1}",
					self.mode,
					scene.cluster_preview(cluster),
					cluster.byte_range,
					cluster.run_index,
					cluster.x,
					cluster.y,
					cluster.width,
					cluster.height,
				)
			}
			EditorMode::Insert => format!(
				"  mode: {}\n  caret byte: {}\n  selection on escape: {}",
				self.mode,
				self.caret,
				scene
					.cluster_before(self.caret)
					.or_else(|| scene.cluster_at_or_after(self.caret))
					.and_then(|index| scene.cluster(index))
					.map(|cluster| scene.cluster_preview(cluster))
					.unwrap_or_else(|| "<none>".to_string())
			),
		}
	}

	fn move_left(&mut self, scene: &LayoutScene) {
		match self.mode {
			EditorMode::Normal => {
				let Some(current) = self.selection_index(scene) else {
					return;
				};

				if let Some(previous) = current.checked_sub(1) {
					self.select_cluster(scene, previous);
				}
			}
			EditorMode::Insert => {
				self.caret = previous_char_boundary(&self.text, self.caret).unwrap_or(0);
				self.preferred_x = None;
			}
		}
	}

	fn move_right(&mut self, scene: &LayoutScene) {
		match self.mode {
			EditorMode::Normal => {
				let Some(current) = self.selection_index(scene) else {
					return;
				};

				if current + 1 < scene.clusters().len() {
					self.select_cluster(scene, current + 1);
				}
			}
			EditorMode::Insert => {
				self.caret = next_char_boundary(&self.text, self.caret).unwrap_or(self.text.len());
				self.preferred_x = None;
			}
		}
	}

	fn move_vertical(&mut self, scene: &LayoutScene, direction: isize) {
		match self.mode {
			EditorMode::Normal => {
				let Some(current) = self.current_selection(scene) else {
					return;
				};
				let preferred_x = self.preferred_x.unwrap_or_else(|| current.center_x());
				let Some(target) = scene.nearest_cluster_on_adjacent_run(current.run_index, preferred_x, direction)
				else {
					return;
				};
				self.select_cluster(scene, target);
				self.preferred_x = Some(preferred_x);
			}
			EditorMode::Insert => {
				let caret = scene.caret_metrics(self.caret);
				let preferred_x = self.preferred_x.unwrap_or(caret.x);
				let Some(target) = scene.nearest_cluster_on_adjacent_run(caret.run_index, preferred_x, direction)
				else {
					return;
				};
				let cluster = &scene.clusters()[target];
				self.caret = if preferred_x > cluster.center_x() {
					cluster.byte_range.end
				} else {
					cluster.byte_range.start
				};
				self.preferred_x = Some(preferred_x);
			}
		}
	}

	fn move_line_edge(&mut self, scene: &LayoutScene, to_start: bool) {
		match self.mode {
			EditorMode::Normal => {
				let Some(current) = self.current_selection(scene) else {
					return;
				};
				let target = if to_start {
					scene.first_cluster_in_run(current.run_index)
				} else {
					scene.last_cluster_in_run(current.run_index)
				};

				if let Some(target) = target {
					self.select_cluster(scene, target);
				}
			}
			EditorMode::Insert => {
				let caret = scene.caret_metrics(self.caret);
				let target = if to_start {
					scene
						.first_cluster_in_run(caret.run_index)
						.map(|index| scene.clusters()[index].byte_range.start)
						.unwrap_or(self.caret)
				} else {
					scene
						.last_cluster_in_run(caret.run_index)
						.map(|index| scene.clusters()[index].byte_range.end)
						.unwrap_or(self.caret)
				};

				self.caret = target;
				self.preferred_x = None;
			}
		}
	}

	fn exit_insert(&mut self, scene: &LayoutScene) {
		self.mode = EditorMode::Normal;
		self.preferred_x = None;

		self.selection = scene
			.cluster_before(self.caret)
			.or_else(|| scene.cluster_at_or_after(self.caret))
			.and_then(|index| scene.cluster(index))
			.map(|cluster| cluster.byte_range.clone());
	}

	fn delete_selection(&mut self, scene: &LayoutScene) -> ApplyResult {
		let Some(selection) = self.current_selection(scene).map(|cluster| cluster.byte_range.clone()) else {
			return ApplyResult {
				changed: false,
				text_edit: None,
			};
		};

		self.text.replace_range(selection.clone(), "");
		self.mode = EditorMode::Normal;
		self.selection = scene
			.cluster_at_or_after(selection.start)
			.or_else(|| scene.cluster_before(selection.start))
			.and_then(|index| scene.cluster(index))
			.map(|cluster| cluster.byte_range.clone());
		self.caret = clamp_char_boundary(&self.text, selection.start);
		self.preferred_x = None;
		ApplyResult {
			changed: true,
			text_edit: Some(TextEdit {
				range: selection,
				inserted: String::new(),
			}),
		}
	}

	fn backspace(&mut self, scene: &LayoutScene) -> ApplyResult {
		match self.mode {
			EditorMode::Normal => self.delete_selection(scene),
			EditorMode::Insert => {
				let Some(previous) = previous_char_boundary(&self.text, self.caret) else {
					return ApplyResult {
						changed: false,
						text_edit: None,
					};
				};

				let range = previous..self.caret;
				self.text.replace_range(previous..self.caret, "");
				self.caret = previous;
				self.preferred_x = None;
				ApplyResult {
					changed: true,
					text_edit: Some(TextEdit {
						range,
						inserted: String::new(),
					}),
				}
			}
		}
	}

	fn delete_forward(&mut self, scene: &LayoutScene) -> ApplyResult {
		match self.mode {
			EditorMode::Normal => self.delete_selection(scene),
			EditorMode::Insert => {
				let Some(next) = next_char_boundary(&self.text, self.caret) else {
					return ApplyResult {
						changed: false,
						text_edit: None,
					};
				};

				let range = self.caret..next;
				self.text.replace_range(self.caret..next, "");
				self.preferred_x = None;
				ApplyResult {
					changed: true,
					text_edit: Some(TextEdit {
						range,
						inserted: String::new(),
					}),
				}
			}
		}
	}

	fn insert_text(&mut self, text: String) -> ApplyResult {
		if text.is_empty() {
			return ApplyResult {
				changed: false,
				text_edit: None,
			};
		}

		if !matches!(self.mode, EditorMode::Insert) {
			self.mode = EditorMode::Insert;
		}

		self.caret = clamp_char_boundary(&self.text, self.caret);
		let range = self.caret..self.caret;
		self.text.insert_str(self.caret, &text);
		self.caret += text.len();
		self.preferred_x = None;
		ApplyResult {
			changed: true,
			text_edit: Some(TextEdit { range, inserted: text }),
		}
	}

	fn select_cluster(&mut self, scene: &LayoutScene, cluster_index: usize) {
		let Some(cluster) = scene.cluster(cluster_index) else {
			return;
		};

		self.mode = EditorMode::Normal;
		self.selection = Some(cluster.byte_range.clone());
		self.caret = cluster.byte_range.start;
		self.preferred_x = Some(cluster.center_x());
	}

	fn selection_index(&self, scene: &LayoutScene) -> Option<usize> {
		self.selection
			.as_ref()
			.and_then(|selection| scene.cluster_index_for_range(selection))
	}

	fn current_selection<'a>(&self, scene: &'a LayoutScene) -> Option<&'a crate::scene::ClusterInfo> {
		self.selection_index(scene).and_then(|index| scene.cluster(index))
	}
}

fn clamp_char_boundary(text: &str, byte: usize) -> usize {
	if byte >= text.len() {
		return text.len();
	}

	let mut boundary = byte;
	while boundary > 0 && !text.is_char_boundary(boundary) {
		boundary -= 1;
	}
	boundary
}

fn previous_char_boundary(text: &str, byte: usize) -> Option<usize> {
	let byte = clamp_char_boundary(text, byte);
	text[..byte].char_indices().last().map(|(index, _)| index)
}

fn next_char_boundary(text: &str, byte: usize) -> Option<usize> {
	let byte = clamp_char_boundary(text, byte);
	text[byte..]
		.char_indices()
		.nth(1)
		.map(|(offset, _)| byte + offset)
		.or_else(|| (byte < text.len()).then_some(text.len()))
}

#[cfg(test)]
mod tests {
	use super::{EditorBuffer, EditorCommand, EditorMode};
	use crate::scene::{CaretMetrics, ClusterInfo, LayoutScene, RunInfo, make_font_system};
	use crate::types::{FontChoice, RenderMode, ShapingChoice, WrapChoice};
	use std::sync::Arc;

	fn scene(clusters: &[(usize, usize, usize, f32)]) -> LayoutScene {
		let cluster_infos = clusters
			.iter()
			.enumerate()
			.map(|(index, (run_index, start, end, x))| ClusterInfo {
				run_index: *run_index,
				glyph_start: index,
				glyph_end: index + 1,
				byte_range: *start..*end,
				x: *x,
				y: (*run_index as f32) * 20.0,
				width: 10.0,
				height: 18.0,
			})
			.collect::<Vec<_>>();

		LayoutScene {
			text: Arc::<str>::from("abc\ndef"),
			font_choice: FontChoice::JetBrainsMono,
			shaping: crate::types::ShapingChoice::Basic,
			wrapping: WrapChoice::Word,
			render_mode: RenderMode::CanvasAndOutlines,
			font_size: 16.0,
			line_height: 20.0,
			max_width: 100.0,
			measured_width: 100.0,
			measured_height: 40.0,
			glyph_count: cluster_infos.len(),
			font_count: 1,
			runs: vec![
				RunInfo {
					line_index: 0,
					rtl: false,
					baseline: 16.0,
					line_top: 0.0,
					line_height: 20.0,
					line_width: 40.0,
					cluster_range: 0..clusters.iter().filter(|(run_index, _, _, _)| *run_index == 0).count(),
					glyphs: Vec::new(),
				},
				RunInfo {
					line_index: 1,
					rtl: false,
					baseline: 36.0,
					line_top: 20.0,
					line_height: 20.0,
					line_width: 40.0,
					cluster_range: clusters.iter().filter(|(run_index, _, _, _)| *run_index == 0).count()
						..cluster_infos.len(),
					glyphs: Vec::new(),
				},
			]
			.into(),
			clusters: cluster_infos.into(),
			warnings: Vec::new().into(),
			draw_canvas_text: true,
			draw_outlines: false,
		}
	}

	#[test]
	fn normal_mode_moves_by_visual_cluster() {
		let scene = scene(&[(0, 0, 1, 0.0), (0, 1, 2, 10.0), (1, 4, 5, 0.0)]);
		let mut editor = EditorBuffer::new("ab\nd");
		editor.sync_with_scene(&scene);

		assert_eq!(editor.view_state().selection, Some(0..1));

		editor.apply(EditorCommand::MoveRight, &scene);
		assert_eq!(editor.view_state().selection, Some(1..2));

		editor.apply(EditorCommand::MoveDown, &scene);
		assert_eq!(editor.view_state().selection, Some(4..5));
	}

	#[test]
	fn insert_mode_backspace_keeps_caret_on_char_boundaries() {
		let scene = scene(&[(0, 0, 1, 0.0), (0, 1, 3, 10.0)]);
		let mut editor = EditorBuffer::new("aé");
		editor.sync_with_scene(&scene);

		editor.apply(EditorCommand::EnterInsertAfter, &scene);
		assert_eq!(editor.view_state().mode, EditorMode::Insert);

		editor.apply(EditorCommand::Backspace, &scene);
		assert_eq!(editor.text(), "é");
		assert_eq!(editor.view_state().caret, 0);
	}

	#[test]
	fn escape_from_insert_returns_to_normal_selection() {
		let scene = scene(&[(0, 0, 1, 0.0), (0, 1, 2, 10.0), (0, 2, 3, 20.0)]);
		let mut editor = EditorBuffer::new("abc");
		editor.sync_with_scene(&scene);

		editor.apply(EditorCommand::EnterInsertAfter, &scene);
		editor.apply(EditorCommand::MoveRight, &scene);
		editor.apply(EditorCommand::ExitInsert, &scene);

		assert_eq!(editor.view_state().mode, EditorMode::Normal);
		assert_eq!(editor.view_state().selection, Some(1..2));
		let CaretMetrics { .. } = scene.caret_metrics(editor.view_state().caret);
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
		let mut editor = EditorBuffer::new(text);
		editor.sync_with_scene(&scene);

		assert_eq!(
			editor
				.view_state()
				.selection
				.as_ref()
				.and_then(|selection| scene.text.get(selection.clone())),
			Some("🙂")
		);

		editor.apply(EditorCommand::MoveDown, &scene);
		assert_eq!(
			editor
				.view_state()
				.selection
				.as_ref()
				.and_then(|selection| scene.text.get(selection.clone())),
			Some("é")
		);

		assert!(editor.apply(EditorCommand::DeleteSelection, &scene).changed);
		assert_eq!(editor.text(), "🙂\n");
	}
}
