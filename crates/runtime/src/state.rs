use {
	glorp_api::{GlorpConfig, GlorpDelta, GlorpRevisions, TextEditView},
	glorp_editor::{DocumentState, HistoryEntry, TextEdit},
};

pub const DEFAULT_LAYOUT_WIDTH: f32 = 540.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionRequest {
	ReplaceDocument(String),
	ApplyEdit(TextEdit),
	Undo,
	Redo,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionDelta {
	pub text_changed: bool,
	pub view_changed: bool,
	pub document_edit: Option<TextEdit>,
}

#[derive(Debug, Clone)]
pub struct DocumentCheckpoint {
	document: DocumentState,
}

#[derive(Debug)]
pub struct DocumentSession {
	document: DocumentState,
}

#[derive(Debug)]
pub struct RuntimeState {
	pub config: GlorpConfig,
	pub session: DocumentSession,
	pub revisions: GlorpRevisions,
}

impl DocumentSession {
	pub fn new(text: &str) -> Self {
		Self {
			document: DocumentState::new(text),
		}
	}

	pub fn execute(&mut self, request: SessionRequest) -> SessionDelta {
		match request {
			SessionRequest::ReplaceDocument(text) => self.execute_replace_document(text),
			SessionRequest::ApplyEdit(edit) => self.execute_apply_edit(edit),
			SessionRequest::Undo => self.execute_undo(),
			SessionRequest::Redo => self.execute_redo(),
		}
	}

	pub fn text(&self) -> &str {
		self.document.text()
	}

	pub fn history_depths(&self) -> (usize, usize) {
		self.document.history_depths()
	}

	pub fn checkpoint(&self) -> DocumentCheckpoint {
		DocumentCheckpoint {
			document: self.document.clone(),
		}
	}

	pub fn restore(&mut self, checkpoint: DocumentCheckpoint) {
		self.document = checkpoint.document;
	}

	fn execute_replace_document(&mut self, text: String) -> SessionDelta {
		let previous_len = self.document.len();
		self.document.reset(text.clone());
		SessionDelta {
			text_changed: true,
			view_changed: false,
			document_edit: Some(TextEdit {
				range: 0..previous_len,
				inserted: text,
			}),
		}
	}

	fn execute_apply_edit(&mut self, edit: TextEdit) -> SessionDelta {
		let inverse = self.document.apply_edit(&edit);
		self.document.record_history(HistoryEntry {
			forward: edit.clone(),
			inverse,
		});
		SessionDelta {
			text_changed: true,
			view_changed: false,
			document_edit: Some(edit),
		}
	}

	fn execute_undo(&mut self) -> SessionDelta {
		let Some(entry) = self.document.undo() else {
			return SessionDelta::default();
		};
		let edit = entry.inverse;
		let _ = self.document.apply_edit(&edit);
		SessionDelta {
			text_changed: true,
			view_changed: false,
			document_edit: Some(edit),
		}
	}

	fn execute_redo(&mut self) -> SessionDelta {
		let Some(entry) = self.document.redo() else {
			return SessionDelta::default();
		};
		let edit = entry.forward;
		let _ = self.document.apply_edit(&edit);
		SessionDelta {
			text_changed: true,
			view_changed: false,
			document_edit: Some(edit),
		}
	}
}

impl RuntimeState {
	pub fn new(config: GlorpConfig, text: &str) -> Self {
		let session = DocumentSession::new(text);

		Self {
			config,
			session,
			revisions: GlorpRevisions { editor: 1, config: 1 },
		}
	}

	pub fn checkpoint(&self) -> RuntimeCheckpoint {
		RuntimeCheckpoint {
			config: self.config.clone(),
			session: self.session.checkpoint(),
			revisions: self.revisions,
		}
	}

	pub fn restore(&mut self, checkpoint: RuntimeCheckpoint) {
		self.config = checkpoint.config;
		self.session.restore(checkpoint.session);
		self.revisions = checkpoint.revisions;
	}

	pub fn delta_from_session(&mut self, session_delta: &SessionDelta) -> GlorpDelta {
		let text_changed = session_delta.text_changed;
		let view_changed = session_delta.view_changed;
		if text_changed || view_changed {
			self.revisions.editor += 1;
		}

		GlorpDelta {
			text_changed,
			view_changed,
			config_changed: false,
		}
	}
}

#[derive(Debug, Clone)]
pub struct RuntimeCheckpoint {
	config: GlorpConfig,
	session: DocumentCheckpoint,
	revisions: GlorpRevisions,
}

pub fn text_edit_view(edit: TextEdit) -> TextEditView {
	TextEditView {
		range: glorp_api::TextRange {
			start: edit.range.start as u64,
			end: edit.range.end as u64,
		},
		inserted: edit.inserted,
	}
}
