use super::{EditorMode, EditorSelection, history::EditorSnapshot};

#[derive(Debug, Clone, Default)]
pub struct EditorSession {
	mode: EditorMode,
	selection: Option<EditorSelection>,
	preferred_x: Option<f32>,
	pointer_anchor: Option<usize>,
}

impl EditorSession {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn mode(&self) -> EditorMode {
		self.mode
	}

	pub fn selection(&self) -> Option<&EditorSelection> {
		self.selection.as_ref()
	}

	pub fn set_mode(&mut self, mode: EditorMode) {
		self.mode = mode;
	}

	pub fn caret(&self) -> usize {
		self.selection.as_ref().map_or(0, EditorSelection::head)
	}

	pub fn preferred_x(&self) -> Option<f32> {
		self.preferred_x
	}

	pub fn pointer_anchor(&self) -> Option<usize> {
		self.pointer_anchor
	}

	pub fn enter_insert(&mut self, selection: Option<EditorSelection>) {
		self.mode = EditorMode::Insert;
		self.selection = selection;
		self.preferred_x = None;
		self.pointer_anchor = None;
	}

	pub fn set_normal_selection(
		&mut self, selection: EditorSelection, preferred_x: Option<f32>, pointer_anchor: Option<usize>,
	) {
		self.mode = EditorMode::Normal;
		self.selection = Some(selection);
		self.preferred_x = preferred_x;
		self.pointer_anchor = pointer_anchor;
	}

	pub fn set_selection(&mut self, selection: Option<EditorSelection>) {
		self.selection = selection;
	}

	pub fn set_preferred_x(&mut self, preferred_x: Option<f32>) {
		self.preferred_x = preferred_x;
	}

	pub fn set_pointer_anchor(&mut self, pointer_anchor: Option<usize>) {
		self.pointer_anchor = pointer_anchor;
	}

	pub fn history_snapshot(&self) -> EditorSnapshot {
		EditorSnapshot {
			mode: self.mode,
			selection: self.selection.clone(),
			preferred_x: self.preferred_x,
		}
	}

	pub fn restore_snapshot(&mut self, snapshot: &EditorSnapshot, document_len: usize) {
		self.mode = snapshot.mode;
		self.selection = snapshot
			.selection
			.as_ref()
			.map(|selection| selection.clamped(document_len));
		self.preferred_x = snapshot.preferred_x;
		self.pointer_anchor = None;
	}
}
