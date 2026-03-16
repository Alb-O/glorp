use {
	crate::{
		editor::{EditorMode, EditorViewState},
		overlay::{EditorOverlayTone, OverlayRectKind},
		perf::{PerfMonitor, PerfSnapshotKey},
		scene::{LayoutScene, debug_snippet},
		types::CanvasTarget,
		ui::{InspectTabProps, PerfTabProps},
	},
	std::{
		cell::{Cell, RefCell},
		ops::Range,
		sync::Arc,
	},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct InspectSidebarKey {
	scene_revision: u64,
	hovered_target: Option<CanvasTarget>,
	selected_target: Option<CanvasTarget>,
	editor_mode: EditorMode,
	selection_start: Option<usize>,
	selection_end: Option<usize>,
	selection_head: Option<usize>,
	pointer_anchor: Option<usize>,
	undo_depth: usize,
	redo_depth: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct PerfSidebarKey {
	scene_revision: u64,
	editor_mode: EditorMode,
	editor_bytes: usize,
	perf: PerfSnapshotKey,
}

#[derive(Debug, Clone)]
pub(super) struct InspectSidebarModel {
	pub(super) key: InspectSidebarKey,
	pub(super) props: Arc<InspectTabProps>,
}

#[derive(Debug, Clone)]
pub(super) struct PerfSidebarModel {
	pub(super) key: PerfSidebarKey,
	pub(super) props: Arc<PerfTabProps>,
}

#[derive(Default)]
pub(super) struct SidebarCache {
	inspect_dirty: Cell<bool>,
	perf_dirty: Cell<bool>,
	inspect: RefCell<Option<InspectSidebarModel>>,
	perf: RefCell<Option<PerfSidebarModel>>,
	#[cfg(test)]
	inspect_builds: Cell<usize>,
	#[cfg(test)]
	perf_builds: Cell<usize>,
}

impl SidebarCache {
	pub(super) fn invalidate_inspect(&self) {
		self.inspect_dirty.set(true);
	}

	pub(super) fn invalidate_perf(&self) {
		self.perf_dirty.set(true);
	}

	pub(super) fn invalidate_scene_derived(&self) {
		self.invalidate_inspect();
		self.invalidate_perf();
	}

	pub(super) fn inspect_model(
		&self, scene_revision: u64, scene: &LayoutScene, editor: &EditorViewState,
		hovered_target: Option<CanvasTarget>, selected_target: Option<CanvasTarget>, undo_depth: usize,
		redo_depth: usize,
	) -> InspectSidebarModel {
		let key = InspectSidebarKey {
			scene_revision,
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

		if !self.inspect_dirty.get() {
			if let Some(model) = self.inspect.borrow().as_ref() {
				if model.key == key {
					return model.clone();
				}
			}
		}

		let model = InspectSidebarModel {
			key,
			props: Arc::new(InspectTabProps {
				warnings: scene.warnings.clone(),
				interaction_details: interaction_details(
					scene,
					editor,
					hovered_target,
					selected_target,
					undo_depth,
					redo_depth,
				),
			}),
		};
		self.inspect_dirty.set(false);
		#[cfg(test)]
		self.inspect_builds.set(self.inspect_builds.get() + 1);
		self.inspect.replace(Some(model.clone()));
		model
	}

	pub(super) fn perf_model(
		&self, scene_revision: u64, scene: &LayoutScene, perf: &PerfMonitor, editor_mode: EditorMode,
		editor_bytes: usize,
	) -> PerfSidebarModel {
		let key = PerfSidebarKey {
			scene_revision,
			editor_mode,
			editor_bytes,
			perf: perf.key(),
		};

		if !self.perf_dirty.get() {
			if let Some(model) = self.perf.borrow().as_ref() {
				if model.key == key {
					return model.clone();
				}
			}
		}

		let model = PerfSidebarModel {
			key,
			props: Arc::new(PerfTabProps {
				dashboard: perf.dashboard(scene, editor_mode, editor_bytes),
			}),
		};
		self.perf_dirty.set(false);
		#[cfg(test)]
		self.perf_builds.set(self.perf_builds.get() + 1);
		self.perf.replace(Some(model.clone()));
		model
	}

	#[cfg(test)]
	pub(super) fn inspect_build_count(&self) -> usize {
		self.inspect_builds.get()
	}

	#[cfg(test)]
	pub(super) fn perf_build_count(&self) -> usize {
		self.perf_builds.get()
	}
}

fn interaction_details(
	scene: &LayoutScene, editor: &EditorViewState, hovered_target: Option<CanvasTarget>,
	selected_target: Option<CanvasTarget>, undo_depth: usize, redo_depth: usize,
) -> String {
	format!(
		"editor\n{}\n\nhover\n{}\n\nselection\n{}",
		editor_selection_details(&scene.text, editor, undo_depth, redo_depth),
		target_details_or_none(scene, hovered_target),
		target_details_or_none(scene, selected_target),
	)
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
	text.get(range.start..range.end)
		.map_or_else(|| "<invalid utf8 slice>".to_string(), debug_snippet)
}

fn target_details_or_none(scene: &LayoutScene, target: Option<CanvasTarget>) -> Arc<str> {
	scene
		.target_details(target)
		.unwrap_or_else(|| Arc::<str>::from("  none"))
}
