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
pub struct EditorCore {
	pub document: DocumentState,
	pub session: EditorSession,
}

impl EditorCore {
	pub fn new(text: impl Into<String>) -> Self {
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
	pub fn text(&self) -> &str {
		self.core.document.text()
	}

	pub const fn mode(&self) -> EditorMode {
		self.core.session.mode()
	}

	pub fn history_depths(&self) -> (usize, usize) {
		self.core.document.history_depths()
	}

	pub fn reset_normal_selection(&mut self) {
		// Normal mode is always anchored to a visible cluster when possible so
		// movement commands can stay purely layout-relative after resets.
		if let Some(cluster) = self.document_layout().cluster(0) {
			let head = cluster.byte_range.start;
			self.core
				.session
				.set_normal_selection(selection_at_cluster(cluster, head), None, Some(head));
		} else {
			self.set_selection(None);
			self.clear_pointer_anchor();
		}
	}

	pub fn select_cluster(&mut self, layout: &DocumentLayout, cluster_index: usize) {
		let Some(cluster) = layout.cluster(cluster_index) else {
			return;
		};

		self.core.session.set_normal_selection(
			selection_at_cluster_start(cluster),
			Some(cluster.center_x()),
			Some(cluster.byte_range.start),
		);
	}

	pub fn active_selection_index(&self, layout: &DocumentLayout) -> Option<usize> {
		let caret = self.selection()?.head();
		layout
			.cluster_at_or_after(caret)
			.or_else(|| layout.cluster_before(caret.saturating_add(1)))
	}

	pub fn active_selection<'layout>(&self, layout: &'layout DocumentLayout) -> Option<&'layout LayoutCluster> {
		layout.cluster(self.active_selection_index(layout)?)
	}

	pub fn history_snapshot(&self) -> EditorSnapshot {
		self.core.session.history_snapshot()
	}

	pub fn restore_snapshot(&mut self, snapshot: &EditorSnapshot) {
		self.core.session.restore_snapshot(snapshot, self.core.document.len());
	}

	pub fn record_history(&mut self, forward: TextEdit, inverse: TextEdit, before: EditorSnapshot) {
		self.core.document.record_history(HistoryEntry {
			forward,
			inverse,
			before,
			after: self.history_snapshot(),
		});
	}

	pub const fn selection(&self) -> Option<&EditorSelection> {
		self.core.session.selection()
	}

	pub fn selection_range(&self) -> Option<Range<usize>> {
		self.selection().map(EditorSelection::range).cloned()
	}

	pub const fn set_selection(&mut self, selection: Option<EditorSelection>) {
		self.core.session.set_selection(selection);
	}

	pub const fn set_mode(&mut self, mode: EditorMode) {
		self.core.session.set_mode(mode);
	}

	pub fn caret(&self) -> usize {
		self.core.session.caret()
	}

	pub const fn preferred_x(&self) -> Option<f32> {
		self.core.session.preferred_x()
	}

	pub const fn set_preferred_x(&mut self, preferred_x: Option<f32>) {
		self.core.session.set_preferred_x(preferred_x);
	}

	pub const fn pointer_anchor(&self) -> Option<usize> {
		self.core.session.pointer_anchor()
	}

	pub const fn set_pointer_anchor(&mut self, pointer_anchor: Option<usize>) {
		self.core.session.set_pointer_anchor(pointer_anchor);
	}

	pub const fn clear_pointer_anchor(&mut self) {
		self.set_pointer_anchor(None);
	}

	pub fn enter_insert_at(&mut self, caret: usize) {
		let layout = self.document_layout();
		self.set_insert_head(&layout, caret);
	}

	#[must_use]
	pub fn insert_selection(layout: &DocumentLayout, head: usize) -> Option<EditorSelection> {
		layout
			.cluster_at_insert_head(head)
			.and_then(|index| layout.cluster(index))
			.map(|cluster| selection_at_cluster(cluster, head))
	}

	pub fn set_insert_head(&mut self, layout: &DocumentLayout, head: usize) {
		self.core.session.enter_insert(Self::insert_selection(layout, head));
	}

	pub fn reset(&mut self, font_system: &mut FontSystem, text: impl Into<String>, config: SceneConfig) {
		let text = text.into();
		self.core.reset(&text);
		self.layout.reset(font_system, &text, config);
		self.reset_normal_selection();
		self.refresh_view_state(None);
	}
}

fn selection_at_cluster(cluster: &LayoutCluster, head: usize) -> EditorSelection {
	EditorSelection::new(cluster.byte_range.clone(), head)
}

fn selection_at_cluster_start(cluster: &LayoutCluster) -> EditorSelection {
	selection_at_cluster(cluster, cluster.byte_range.start)
}
