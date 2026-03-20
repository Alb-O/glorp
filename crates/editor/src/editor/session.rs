use super::{EditorMode, EditorSelection};

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

	pub const fn mode(&self) -> EditorMode {
		self.mode
	}

	pub const fn selection(&self) -> Option<&EditorSelection> {
		self.selection.as_ref()
	}

	pub const fn set_mode(&mut self, mode: EditorMode) {
		self.mode = mode;
	}

	pub fn caret(&self) -> usize {
		self.selection.as_ref().map_or(0, EditorSelection::head)
	}

	pub const fn preferred_x(&self) -> Option<f32> {
		self.preferred_x
	}

	pub const fn pointer_anchor(&self) -> Option<usize> {
		self.pointer_anchor
	}

	pub const fn enter_insert(&mut self, selection: Option<EditorSelection>) {
		self.mode = EditorMode::Insert;
		self.selection = selection;
		self.preferred_x = None;
		self.pointer_anchor = None;
	}

	pub const fn set_normal_selection(
		&mut self, selection: EditorSelection, preferred_x: Option<f32>, pointer_anchor: Option<usize>,
	) {
		self.mode = EditorMode::Normal;
		self.selection = Some(selection);
		self.preferred_x = preferred_x;
		self.pointer_anchor = pointer_anchor;
	}

	pub const fn set_selection(&mut self, selection: Option<EditorSelection>) {
		self.selection = selection;
	}

	pub const fn set_preferred_x(&mut self, preferred_x: Option<f32>) {
		self.preferred_x = preferred_x;
	}

	pub const fn set_pointer_anchor(&mut self, pointer_anchor: Option<usize>) {
		self.pointer_anchor = pointer_anchor;
	}
}
