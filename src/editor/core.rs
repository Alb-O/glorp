use {
	super::{
		EditorEngine, EditorMode, EditorSelection, TextEdit,
		document::DocumentState,
		history::{EditorSnapshot, HistoryEntry},
		session::EditorSession,
	},
	crate::scene::{DocumentLayout, LayoutCluster, SceneConfig},
	cosmic_text::FontSystem,
	std::ops::Range,
};

#[derive(Debug, Clone)]
pub(super) struct EditorCore {
	pub(super) document: DocumentState,
	pub(super) session: EditorSession,
}

impl EditorCore {
	pub(super) fn new(text: impl Into<String>) -> Self {
		Self {
			document: DocumentState::new(text),
			session: EditorSession::new(),
		}
	}

	fn reset(&mut self, text: &str) {
		self.document.reset(text);
		self.session = EditorSession::new();
	}
}

impl EditorEngine {
	pub(crate) fn text(&self) -> &str {
		self.core.document.text()
	}

	pub(crate) fn mode(&self) -> EditorMode {
		self.core.session.mode()
	}

	pub(crate) fn history_depths(&self) -> (usize, usize) {
		self.core.document.history_depths()
	}

	pub(super) fn reset_normal_selection(&mut self) {
		// Normal mode is always anchored to a visible cluster when possible so
		// movement commands can stay purely layout-relative after resets.
		if let Some(cluster) = self.document_layout().cluster(0) {
			let head = cluster.byte_range.start;
			self.core.session.set_normal_selection(
				EditorSelection::new(cluster.byte_range.clone(), head),
				None,
				Some(head),
			);
		} else {
			self.set_selection(None);
			self.clear_pointer_anchor();
		}
	}

	pub(super) fn select_cluster(&mut self, layout: &DocumentLayout, cluster_index: usize) {
		let Some(cluster) = layout.cluster(cluster_index) else {
			return;
		};

		self.core.session.set_normal_selection(
			EditorSelection::new(cluster.byte_range.clone(), cluster.byte_range.start),
			Some(cluster.center_x()),
			Some(cluster.byte_range.start),
		);
	}

	pub(super) fn active_selection_index(&self, layout: &DocumentLayout) -> Option<usize> {
		self.selection()?;

		layout
			.cluster_at_or_after(self.caret())
			.or_else(|| layout.cluster_before(self.caret().saturating_add(1)))
	}

	pub(super) fn active_selection<'a>(&self, layout: &'a DocumentLayout) -> Option<&'a LayoutCluster> {
		self.active_selection_index(layout)
			.and_then(|index| layout.cluster(index))
	}

	pub(super) fn history_snapshot(&self) -> EditorSnapshot {
		self.core.session.history_snapshot()
	}

	pub(super) fn restore_snapshot(&mut self, snapshot: &EditorSnapshot) {
		self.core.session.restore_snapshot(snapshot, self.core.document.len());
	}

	pub(super) fn record_history(&mut self, forward: TextEdit, inverse: TextEdit, before: EditorSnapshot) {
		self.core.document.record_history(HistoryEntry {
			forward,
			inverse,
			before,
			after: self.history_snapshot(),
		});
	}

	pub(super) fn selection(&self) -> Option<&EditorSelection> {
		self.core.session.selection()
	}

	pub(super) fn selection_range(&self) -> Option<Range<usize>> {
		self.selection().map(|selection| selection.range.clone())
	}

	pub(super) fn set_selection(&mut self, selection: Option<EditorSelection>) {
		self.core.session.set_selection(selection);
	}

	pub(super) fn set_mode(&mut self, mode: EditorMode) {
		self.core.session.set_mode(mode);
	}

	pub(super) fn caret(&self) -> usize {
		self.core.session.caret()
	}

	pub(super) fn preferred_x(&self) -> Option<f32> {
		self.core.session.preferred_x()
	}

	pub(super) fn set_preferred_x(&mut self, preferred_x: Option<f32>) {
		self.core.session.set_preferred_x(preferred_x);
	}

	pub(super) fn pointer_anchor(&self) -> Option<usize> {
		self.core.session.pointer_anchor()
	}

	pub(super) fn set_pointer_anchor(&mut self, pointer_anchor: Option<usize>) {
		self.core.session.set_pointer_anchor(pointer_anchor);
	}

	pub(super) fn clear_pointer_anchor(&mut self) {
		self.set_pointer_anchor(None);
	}

	pub(super) fn enter_insert_at(&mut self, caret: usize) {
		let layout = self.document_layout();
		self.enter_insert_with_layout(&layout, caret);
	}

	pub(super) fn enter_insert_with_layout(&mut self, layout: &DocumentLayout, caret: usize) {
		self.set_insert_head(layout, caret);
	}

	pub(super) fn insert_selection(layout: &DocumentLayout, head: usize) -> Option<EditorSelection> {
		layout
			.cluster_at_insert_head(head)
			.and_then(|index| layout.cluster(index))
			.map(|cluster| EditorSelection::new(cluster.byte_range.clone(), head))
	}

	pub(super) fn set_insert_head(&mut self, layout: &DocumentLayout, head: usize) {
		self.core.session.enter_insert(Self::insert_selection(layout, head));
	}

	pub(crate) fn reset(&mut self, font_system: &mut FontSystem, text: impl Into<String>, config: SceneConfig) {
		let text = text.into();
		self.core.reset(&text);
		self.layout.reset(font_system, &text, config);
		self.reset_normal_selection();
		self.refresh_view_state(None);
	}
}
