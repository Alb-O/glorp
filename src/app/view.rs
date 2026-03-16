use {
	super::{Playground, state::ShellPane},
	crate::{
		editor::{EditorMode, EditorViewState},
		overlay::{EditorOverlayTone, OverlayRectKind},
		perf::PerfSnapshotKey,
		scene::LayoutScene,
		types::{Message, ShellMessage, SidebarTab},
		ui::{
			CanvasPaneProps, ControlsTabProps, InspectTabProps, PerfTabProps, SidebarProps, is_stacked_shell,
			view_canvas_pane, view_controls_tab, view_inspect_tab, view_perf_tab, view_sidebar, view_stacked_shell,
		},
	},
	iced::{
		Element, Length,
		widget::{lazy, pane_grid, responsive},
	},
	std::{fmt::Write as _, ops::Range, sync::Arc},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct InspectSidebarKey {
	scene_revision: u64,
	hovered_target: Option<crate::types::CanvasTarget>,
	selected_target: Option<crate::types::CanvasTarget>,
	editor_mode: EditorMode,
	selection_start: Option<usize>,
	selection_end: Option<usize>,
	selection_head: Option<usize>,
	pointer_anchor: Option<usize>,
	undo_depth: usize,
	redo_depth: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct PerfSidebarKey {
	scene_revision: u64,
	editor_mode: EditorMode,
	editor_bytes: usize,
	perf: PerfSnapshotKey,
}

impl Playground {
	pub(crate) fn view(&self) -> Element<'_, Message> {
		responsive(|size| {
			if is_stacked_shell(size) {
				let sidebar = self.view_sidebar(true);
				let canvas = self.view_canvas(true);
				return view_stacked_shell(sidebar, canvas);
			}

			let grid = pane_grid(&self.shell.chrome, |_, pane, _| {
				pane_grid::Content::new(match pane {
					ShellPane::Sidebar => self.view_sidebar(false),
					ShellPane::Canvas => self.view_canvas(false),
				})
			})
			.width(Length::Fill)
			.height(Length::Fill)
			.spacing(12)
			.min_size(220)
			.on_resize(12, |event| Message::Shell(ShellMessage::PaneResized(event)));

			iced::widget::container(grid)
				.padding(16)
				.width(Length::Fill)
				.height(Length::Fill)
				.into()
		})
		.width(Length::Fill)
		.height(Length::Fill)
		.into()
	}

	pub fn headless_view(&self) -> Element<'_, ()> {
		self.view().map(|_| ())
	}

	fn view_sidebar(&self, stacked: bool) -> Element<'_, Message> {
		let (undo_depth, redo_depth) = self.session.history_depths();
		view_sidebar(SidebarProps {
			active_tab: self.sidebar.active_tab,
			editor_mode: self.session.mode(),
			editor_bytes: self.session.text().len(),
			undo_depth,
			redo_depth,
			body: self.view_sidebar_body(undo_depth, redo_depth),
			stacked,
		})
	}

	fn view_canvas(&self, stacked: bool) -> Element<'static, Message> {
		let inspect_overlays = if self.sidebar.active_tab == SidebarTab::Inspect {
			self.session.inspect_overlay_primitives(
				self.sidebar.hovered_target,
				self.sidebar.selected_target,
				self.viewport.layout_width,
				self.controls.show_hitboxes,
			)
		} else {
			Arc::from([])
		};

		view_canvas_pane(CanvasPaneProps {
			scene: self.session.scene().clone(),
			text_layer: self.session.text_layer_state(),
			layout_width: self.viewport.layout_width,
			show_baselines: self.controls.show_baselines,
			show_hitboxes: self.controls.show_hitboxes,
			inspect_overlays,
			editor: self.session.view_state(),
			focused: self.viewport.canvas_focused,
			scene_revision: self.viewport.scene_revision,
			scroll: self.viewport.canvas_scroll,
			perf: self.perf.sink(),
			stacked,
		})
	}

	fn view_sidebar_body(&self, undo_depth: usize, redo_depth: usize) -> Element<'static, Message> {
		match self.sidebar.active_tab {
			SidebarTab::Controls => view_controls_tab(ControlsTabProps {
				preset: self.controls.preset,
				font: self.controls.font,
				shaping: self.controls.shaping,
				wrapping: self.controls.wrapping,
				render_mode: self.controls.render_mode,
				font_size: self.controls.font_size,
				line_height: self.controls.line_height,
				show_baselines: self.controls.show_baselines,
				show_hitboxes: self.controls.show_hitboxes,
			}),
			SidebarTab::Inspect => self.view_inspect_sidebar(undo_depth, redo_depth),
			SidebarTab::Perf => self.view_perf_sidebar(),
		}
	}

	fn view_inspect_sidebar(&self, undo_depth: usize, redo_depth: usize) -> Element<'static, Message> {
		let scene = self.session.scene().clone();
		let editor = self.session.view_state();
		let hovered_target = self.sidebar.hovered_target;
		let selected_target = self.sidebar.selected_target;
		let key = InspectSidebarKey {
			scene_revision: self.viewport.scene_revision,
			hovered_target,
			selected_target,
			editor_mode: editor.mode,
			selection_start: editor.selection.as_ref().map(|range| range.start),
			selection_end: editor.selection.as_ref().map(|range| range.end),
			selection_head: editor.selection_head,
			pointer_anchor: editor.pointer_anchor,
			undo_depth,
			redo_depth,
		};

		lazy(key, move |_| {
			view_inspect_tab(InspectTabProps {
				warnings: scene.warnings.clone(),
				interaction_details: interaction_details(
					&scene,
					&editor,
					hovered_target,
					selected_target,
					undo_depth,
					redo_depth,
				),
			})
		})
		.into()
	}

	fn view_perf_sidebar(&self) -> Element<'static, Message> {
		let scene = self.session.scene().clone();
		let perf = self.perf.snapshot();
		let editor_mode = self.session.mode();
		let editor_bytes = self.session.text().len();
		let key = PerfSidebarKey {
			scene_revision: self.viewport.scene_revision,
			editor_mode,
			editor_bytes,
			perf: perf.key(),
		};

		lazy(key, move |_| {
			view_perf_tab(PerfTabProps {
				dashboard: perf.dashboard(&scene, editor_mode, editor_bytes),
			})
		})
		.into()
	}
}

fn interaction_details(
	scene: &LayoutScene, editor: &EditorViewState, hovered_target: Option<crate::types::CanvasTarget>,
	selected_target: Option<crate::types::CanvasTarget>, undo_depth: usize, redo_depth: usize,
) -> String {
	let mut details = String::new();
	let _ = writeln!(details, "editor");
	let _ = writeln!(
		details,
		"{}",
		editor_selection_details(&scene.text, editor, undo_depth, redo_depth)
	);
	let _ = writeln!(details);
	let _ = writeln!(details, "hover");
	let _ = writeln!(
		details,
		"{}",
		scene
			.target_details(hovered_target)
			.unwrap_or_else(|| Arc::<str>::from("  none"))
	);
	let _ = writeln!(details);
	let _ = writeln!(details, "selection");
	let _ = writeln!(
		details,
		"{}",
		scene
			.target_details(selected_target)
			.unwrap_or_else(|| Arc::<str>::from("  none"))
	);
	details
}

fn editor_selection_details(text: &str, editor: &EditorViewState, undo_depth: usize, redo_depth: usize) -> String {
	let selection_rects = editor.overlay_count(OverlayRectKind::EditorSelection(EditorOverlayTone::from(editor.mode)));

	match (editor.mode, editor.selection.as_ref()) {
		(EditorMode::Normal, None) => format!("  mode: {}\n  selection: none", editor.mode),
		(EditorMode::Normal, Some(selection)) => format!(
			"  mode: {}\n  bytes: {:?}\n  text: {}\n  rects: {}\n  active byte: {}\n  anchor byte: {}\n  undo/redo: {}/{}",
			editor.mode,
			selection,
			preview_range(text, selection),
			selection_rects,
			editor.selection_head.unwrap_or(selection.start),
			editor.pointer_anchor.unwrap_or(selection.start),
			undo_depth,
			redo_depth,
		),
		(EditorMode::Insert, None) => format!(
			"  mode: {}\n  selection: none\n  undo/redo: {undo_depth}/{redo_depth}",
			editor.mode
		),
		(EditorMode::Insert, Some(selection)) => format!(
			"  mode: {}\n  bytes: {:?}\n  text: {}\n  rects: {}\n  head byte: {}\n  undo/redo: {}/{}",
			editor.mode,
			selection,
			preview_range(text, selection),
			selection_rects,
			editor.selection_head.unwrap_or(selection.start),
			undo_depth,
			redo_depth,
		),
	}
}

fn preview_range(text: &str, range: &Range<usize>) -> String {
	text.get(range.clone())
		.map(debug_snippet)
		.unwrap_or_else(|| "<invalid utf8 slice>".to_string())
}

fn debug_snippet(text: &str) -> String {
	let escaped = text.chars().flat_map(char::escape_default).collect::<String>();
	if escaped.is_empty() {
		"<empty>".to_string()
	} else {
		format!("\"{escaped}\"")
	}
}
