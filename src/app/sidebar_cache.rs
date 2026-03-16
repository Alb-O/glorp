use {
	super::sidebar_data::InspectSidebarData,
	crate::{
		editor::{EditorMode, EditorViewState},
		overlay::OverlayRectKind,
		perf::{PerfDashboard, PerfMonitor, PerfSnapshotKey},
		presentation::{DerivedScenePresentation, EditorPresentation},
		scene::{DocumentLayout, debug_snippet},
		types::CanvasTarget,
	},
	std::{
		cell::{Cell, RefCell},
		ops::Range,
		sync::Arc,
	},
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
	inspect_dirty: Cell<bool>,
	perf_dirty: Cell<bool>,
	inspect: RefCell<Option<CachedEntry<InspectSidebarKey, InspectSidebarData>>>,
	perf: RefCell<Option<CachedEntry<PerfSidebarKey, PerfDashboard>>>,
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

	pub(super) fn inspect_data(&self, args: InspectSidebarArgs<'_>) -> Arc<InspectSidebarData> {
		let key = args.key();

		if let Some(data) = cached_data(&self.inspect, &self.inspect_dirty, key) {
			return data;
		}

		let data = Arc::new(InspectSidebarData {
			warnings: args.scene.layout.warnings.clone(),
			interaction_details: interaction_details(
				args.editor,
				args.scene,
				args.text,
				args.hovered_target,
				args.selected_target,
				args.undo_depth,
				args.redo_depth,
			),
		});
		self.inspect_dirty.set(false);
		#[cfg(test)]
		self.inspect_builds.set(self.inspect_builds.get() + 1);
		self.inspect.replace(Some(CachedEntry {
			key,
			data: data.clone(),
		}));
		data
	}

	pub(super) fn perf_dashboard(
		&self, editor: &EditorPresentation, scene: &DerivedScenePresentation, perf: &PerfMonitor,
	) -> Arc<PerfDashboard> {
		let key = PerfSidebarKey {
			editor_revision: editor.revision,
			scene_revision: scene.revision,
			editor_mode: editor.mode(),
			editor_bytes: editor.editor_bytes(),
			perf: perf.key(),
		};

		if let Some(data) = cached_data(&self.perf, &self.perf_dirty, key) {
			return data;
		}

		let data = Arc::new(perf.dashboard(scene.layout.as_ref(), editor.mode(), editor.editor_bytes()));
		self.perf_dirty.set(false);
		#[cfg(test)]
		self.perf_builds.set(self.perf_builds.get() + 1);
		self.perf.replace(Some(CachedEntry {
			key,
			data: data.clone(),
		}));
		data
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
	pub(super) scene: &'a DerivedScenePresentation,
	pub(super) text: &'a str,
	pub(super) hovered_target: Option<CanvasTarget>,
	pub(super) selected_target: Option<CanvasTarget>,
	pub(super) undo_depth: usize,
	pub(super) redo_depth: usize,
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
			undo_depth: self.undo_depth,
			redo_depth: self.redo_depth,
		}
	}
}

fn cached_data<K, V>(cache: &RefCell<Option<CachedEntry<K, V>>>, dirty: &Cell<bool>, key: K) -> Option<Arc<V>>
where
	K: Copy + Eq, {
	if !dirty.get()
		&& let Some(entry) = cache.borrow().as_ref()
		&& entry.key == key
	{
		return Some(entry.data.clone());
	}

	None
}

fn interaction_details(
	editor: &EditorPresentation, scene: &DerivedScenePresentation, text: &str, hovered_target: Option<CanvasTarget>,
	selected_target: Option<CanvasTarget>, undo_depth: usize, redo_depth: usize,
) -> Arc<str> {
	Arc::<str>::from(format!(
		"editor\n{}\n\nhover\n{}\n\nselection\n{}",
		editor_selection_details(text, &editor.editor, undo_depth, redo_depth),
		target_details_or_none(scene.layout.as_ref(), hovered_target),
		target_details_or_none(scene.layout.as_ref(), selected_target),
	))
}

fn editor_selection_details(text: &str, editor: &EditorViewState, undo_depth: usize, redo_depth: usize) -> String {
	let selection_rects = editor.overlay_count(OverlayRectKind::EditorSelection);

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

fn target_details_or_none(layout: &DocumentLayout, target: Option<CanvasTarget>) -> Arc<str> {
	layout
		.target_details(target)
		.unwrap_or_else(|| Arc::<str>::from("  none"))
}
