#[cfg(test)]
use std::cell::Cell;

use {
	super::sidebar_data::InspectSidebarData,
	crate::{
		editor::EditorMode,
		overlay::OverlayRectKind,
		perf::{PerfDashboard, PerfMonitor, PerfSnapshotKey},
		presentation::{EditorPresentation, ScenePresentation},
		scene::{DocumentLayout, debug_snippet},
		types::CanvasTarget,
	},
	std::{cell::RefCell, ops::Range, sync::Arc},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct InspectSidebarKey {
	editor_revision: u64,
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
	editor_revision: u64,
	scene_revision: u64,
	editor_mode: EditorMode,
	editor_bytes: usize,
	perf: PerfSnapshotKey,
}

#[derive(Debug, Clone)]
struct CachedEntry<K, V> {
	key: K,
	data: Arc<V>,
}

#[derive(Default)]
pub(super) struct SidebarCache {
	inspect: RefCell<Option<CachedEntry<InspectSidebarKey, InspectSidebarData>>>,
	perf: RefCell<Option<CachedEntry<PerfSidebarKey, PerfDashboard>>>,
	#[cfg(test)]
	inspect_builds: Cell<usize>,
	#[cfg(test)]
	perf_builds: Cell<usize>,
}

impl SidebarCache {
	pub(super) fn inspect_data(&self, args: InspectSidebarArgs<'_>) -> Arc<InspectSidebarData> {
		let key = args.key();
		cached_or_build(&self.inspect, key, || {
			#[cfg(test)]
			self.inspect_builds.set(self.inspect_builds.get() + 1);

			InspectSidebarData {
				warnings: args.scene.layout.warnings.clone(),
				interaction_details: interaction_details(
					args.editor,
					args.scene,
					args.text,
					args.hovered_target,
					args.selected_target,
				),
			}
		})
	}

	pub(super) fn perf_dashboard(
		&self, editor: &EditorPresentation, scene: &ScenePresentation, perf: &PerfMonitor,
	) -> Arc<PerfDashboard> {
		let key = PerfSidebarKey {
			editor_revision: editor.revision,
			scene_revision: scene.revision,
			editor_mode: editor.editor.mode,
			editor_bytes: editor.editor_bytes,
			perf: perf.key(),
		};

		cached_or_build(&self.perf, key, || {
			#[cfg(test)]
			self.perf_builds.set(self.perf_builds.get() + 1);

			perf.dashboard(scene.layout.as_ref(), editor.editor.mode, editor.editor_bytes)
		})
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

#[derive(Clone, Copy)]
pub(super) struct InspectSidebarArgs<'a> {
	pub(super) editor: &'a EditorPresentation,
	pub(super) scene: &'a ScenePresentation,
	pub(super) text: &'a str,
	pub(super) hovered_target: Option<CanvasTarget>,
	pub(super) selected_target: Option<CanvasTarget>,
}

impl InspectSidebarArgs<'_> {
	fn key(self) -> InspectSidebarKey {
		let selection = self.editor.editor.selection.as_ref();
		InspectSidebarKey {
			editor_revision: self.editor.revision,
			scene_revision: self.scene.revision,
			hovered_target: self.hovered_target,
			selected_target: self.selected_target,
			editor_mode: self.editor.editor.mode,
			selection_start: selection.map(|range| range.start),
			selection_end: selection.map(|range| range.end),
			selection_head: self.editor.editor.selection_head,
			pointer_anchor: self.editor.editor.pointer_anchor,
			undo_depth: self.editor.undo_depth,
			redo_depth: self.editor.redo_depth,
		}
	}
}

fn cached_or_build<K, V>(cache: &RefCell<Option<CachedEntry<K, V>>>, key: K, build: impl FnOnce() -> V) -> Arc<V>
where
	K: Copy + Eq, {
	if let Some(entry) = cache.borrow().as_ref()
		&& entry.key == key
	{
		return Arc::clone(&entry.data);
	}

	let data = Arc::new(build());
	cache.replace(Some(CachedEntry {
		key,
		data: Arc::clone(&data),
	}));
	data
}

fn interaction_details(
	editor: &EditorPresentation, scene: &ScenePresentation, text: &str, hovered_target: Option<CanvasTarget>,
	selected_target: Option<CanvasTarget>,
) -> Arc<str> {
	let layout = scene.layout.as_ref();
	Arc::<str>::from(format!(
		"editor\n{}\n\nhover\n{}\n\nselection\n{}",
		editor_selection_details(text, editor),
		target_details_or_none(layout, hovered_target),
		target_details_or_none(layout, selected_target),
	))
}

fn editor_selection_details(text: &str, editor: &EditorPresentation) -> String {
	let view = &editor.editor;
	let selection_rects = view.overlay_count(OverlayRectKind::EditorSelection);
	let Some(selection) = view.selection.as_ref() else {
		return match view.mode {
			EditorMode::Normal => format!("  mode: {}\n  selection: none", view.mode),
			EditorMode::Insert => format!(
				"  mode: {}\n  selection: none\n  undo/redo: {}/{}",
				view.mode, editor.undo_depth, editor.redo_depth
			),
		};
	};

	match view.mode {
		EditorMode::Normal => format!(
			"  mode: {}\n  bytes: {:?}\n  text: {}\n  rects: {}\n  active byte: {}\n  anchor byte: {}\n  undo/redo: {}/{}",
			view.mode,
			selection,
			preview_range(text, selection),
			selection_rects,
			view.selection_head.unwrap_or(selection.start),
			view.pointer_anchor.unwrap_or(selection.start),
			editor.undo_depth,
			editor.redo_depth,
		),
		EditorMode::Insert => format!(
			"  mode: {}\n  bytes: {:?}\n  text: {}\n  rects: {}\n  head byte: {}\n  undo/redo: {}/{}",
			view.mode,
			selection,
			preview_range(text, selection),
			selection_rects,
			view.selection_head.unwrap_or(selection.start),
			editor.undo_depth,
			editor.redo_depth,
		),
	}
}

fn preview_range(text: &str, range: &Range<usize>) -> String {
	text.get(range.clone())
		.map_or_else(|| "<invalid utf8 slice>".to_string(), debug_snippet)
}

fn target_details_or_none(layout: &DocumentLayout, target: Option<CanvasTarget>) -> Arc<str> {
	layout.target_details(target).unwrap_or_else(|| "  none".into())
}
