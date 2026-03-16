use {
	super::sidebar_data::{InspectSidebarData, PerfSidebarData},
	crate::{
		editor::{EditorMode, EditorViewState},
		overlay::{EditorOverlayTone, OverlayRectKind},
		perf::{PerfMonitor, PerfSnapshotKey},
		presentation::DocumentPresentation,
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
	presentation_revision: u64,
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
	presentation_revision: u64,
	editor_mode: EditorMode,
	editor_bytes: usize,
	perf: PerfSnapshotKey,
}

#[derive(Debug, Clone)]
pub(super) struct InspectSidebarModel {
	pub(super) key: InspectSidebarKey,
	pub(super) data: Arc<InspectSidebarData>,
}

#[derive(Debug, Clone)]
pub(super) struct PerfSidebarModel {
	pub(super) key: PerfSidebarKey,
	pub(super) data: Arc<PerfSidebarData>,
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

	pub(super) fn inspect_model(&self, args: InspectSidebarArgs<'_>) -> InspectSidebarModel {
		let key = args.key();

		if let Some(model) = cached_model(&self.inspect, &self.inspect_dirty, key, |model| model.key) {
			return model;
		}

		let model = InspectSidebarModel {
			key,
			data: Arc::new(InspectSidebarData {
				warnings: args.presentation.layout.warnings.clone(),
				interaction_details: interaction_details(
					args.presentation,
					args.hovered_target,
					args.selected_target,
					args.undo_depth,
					args.redo_depth,
				),
			}),
		};
		self.inspect_dirty.set(false);
		#[cfg(test)]
		self.inspect_builds.set(self.inspect_builds.get() + 1);
		self.inspect.replace(Some(model.clone()));
		model
	}

	pub(super) fn perf_model(&self, presentation: &DocumentPresentation, perf: &PerfMonitor) -> PerfSidebarModel {
		let key = PerfSidebarKey {
			presentation_revision: presentation.revision,
			editor_mode: presentation.mode(),
			editor_bytes: presentation.editor_bytes(),
			perf: perf.key(),
		};

		if let Some(model) = cached_model(&self.perf, &self.perf_dirty, key, |model| model.key) {
			return model;
		}

		let model = PerfSidebarModel {
			key,
			data: Arc::new(PerfSidebarData {
				dashboard: Arc::new(perf.dashboard(
					presentation.layout.as_ref(),
					presentation.mode(),
					presentation.editor_bytes(),
				)),
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

#[derive(Clone, Copy)]
pub(super) struct InspectSidebarArgs<'a> {
	pub(super) presentation: &'a DocumentPresentation,
	pub(super) hovered_target: Option<CanvasTarget>,
	pub(super) selected_target: Option<CanvasTarget>,
	pub(super) undo_depth: usize,
	pub(super) redo_depth: usize,
}

impl InspectSidebarArgs<'_> {
	fn key(self) -> InspectSidebarKey {
		InspectSidebarKey {
			presentation_revision: self.presentation.revision,
			hovered_target: self.hovered_target,
			selected_target: self.selected_target,
			editor_mode: self.presentation.editor.mode,
			selection_start: self.presentation.editor.selection.as_ref().map(|range| range.start),
			selection_end: self.presentation.editor.selection.as_ref().map(|range| range.end),
			selection_head: self.presentation.editor.selection_head,
			pointer_anchor: self.presentation.editor.pointer_anchor,
			undo_depth: self.undo_depth,
			redo_depth: self.redo_depth,
		}
	}
}

fn cached_model<T, K>(model: &RefCell<Option<T>>, dirty: &Cell<bool>, key: K, key_of: impl Fn(&T) -> K) -> Option<T>
where
	T: Clone,
	K: Copy + Eq, {
	if !dirty.get()
		&& let Some(model) = model.borrow().as_ref()
		&& key_of(model) == key
	{
		return Some(model.clone());
	}

	None
}

fn interaction_details(
	presentation: &DocumentPresentation, hovered_target: Option<CanvasTarget>, selected_target: Option<CanvasTarget>,
	undo_depth: usize, redo_depth: usize,
) -> Arc<str> {
	Arc::<str>::from(format!(
		"editor\n{}\n\nhover\n{}\n\nselection\n{}",
		editor_selection_details(presentation.text(), &presentation.editor, undo_depth, redo_depth),
		target_details_or_none(presentation.layout.as_ref(), hovered_target),
		target_details_or_none(presentation.layout.as_ref(), selected_target),
	))
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

fn target_details_or_none(layout: &DocumentLayout, target: Option<CanvasTarget>) -> Arc<str> {
	layout
		.target_details(target)
		.unwrap_or_else(|| Arc::<str>::from("  none"))
}
