use {
	crate::{perf::PerfProjection, scene::scene_config_from_runtime},
	glorp_api::{GlorpConfig, GlorpDelta, GlorpRevisions, TextEditView},
	glorp_editor::{EditorEngine, EditorIntent, ScenePresentation, TextEdit, make_font_system},
	std::time::Instant,
};

pub const DEFAULT_LAYOUT_WIDTH: f32 = 540.0;

#[derive(Debug, Clone, PartialEq)]
pub enum SessionRequest {
	ReplaceDocument(String),
	SyncConfig,
	ApplyEditorIntent(EditorIntent),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionDelta {
	pub text_changed: bool,
	pub view_changed: bool,
	pub document_edit: Option<TextEdit>,
}

#[derive(Debug, Clone)]
pub struct DocumentCheckpoint {
	editor: EditorEngine,
	scene: Option<ScenePresentation>,
	scene_revision: u64,
}

#[derive(Debug)]
pub struct DocumentSession {
	editor: EditorEngine,
	scene: Option<ScenePresentation>,
	scene_revision: u64,
	font_system: cosmic_text::FontSystem,
}

#[derive(Debug)]
pub struct RuntimeState {
	pub config: GlorpConfig,
	pub session: DocumentSession,
	pub revisions: GlorpRevisions,
	pub perf: PerfProjection,
}

impl DocumentSession {
	pub fn new(text: &str, config: &GlorpConfig, layout_width: f32) -> Self {
		let mut font_system = make_font_system();
		let scene_config = scene_config_from_runtime(config, layout_width);
		let editor = EditorEngine::new(&mut font_system, text, scene_config);

		Self {
			editor,
			scene: None,
			scene_revision: 1,
			font_system,
		}
	}

	pub fn execute(&mut self, request: SessionRequest, config: &GlorpConfig, layout_width: f32) -> SessionDelta {
		match request {
			SessionRequest::ReplaceDocument(text) => self.execute_replace_document(&text, config, layout_width),
			SessionRequest::SyncConfig => self.execute_sync_config(config, layout_width),
			SessionRequest::ApplyEditorIntent(intent) => self.execute_editor_intent(intent),
		}
	}

	pub fn scene_summary(&self) -> crate::GuiSceneSummary {
		crate::GuiSceneSummary {
			revision: self.scene_revision,
		}
	}

	pub fn text(&self) -> &str {
		self.editor.text()
	}

	pub fn editor(&self) -> &EditorEngine {
		&self.editor
	}

	pub fn editor_mut(&mut self) -> &mut EditorEngine {
		&mut self.editor
	}

	pub fn layout_width(&self) -> f32 {
		self.editor.layout_width()
	}

	pub fn history_depths(&self) -> (usize, usize) {
		self.editor.history_depths()
	}

	pub fn fetch_scene(&mut self) -> (ScenePresentation, Option<std::time::Duration>) {
		if let Some(scene) = self.scene.clone() {
			return (scene, None);
		}

		let started = Instant::now();
		let scene = ScenePresentation::new(self.scene_revision, self.editor.shared_document_layout());
		let duration = started.elapsed();
		self.scene = Some(scene.clone());
		(scene, Some(duration))
	}

	pub fn checkpoint(&self) -> DocumentCheckpoint {
		DocumentCheckpoint {
			editor: self.editor.clone(),
			scene: self.scene.clone(),
			scene_revision: self.scene_revision,
		}
	}

	pub fn restore(&mut self, checkpoint: DocumentCheckpoint) {
		self.editor = checkpoint.editor;
		self.scene = checkpoint.scene;
		self.scene_revision = checkpoint.scene_revision;
		self.font_system = make_font_system();
	}

	pub fn sync_layout_width(&mut self, config: &GlorpConfig, layout_width: f32) {
		let _ = self.execute(SessionRequest::SyncConfig, config, layout_width);
	}

	fn execute_replace_document(&mut self, text: &str, config: &GlorpConfig, layout_width: f32) -> SessionDelta {
		let previous_len = self.editor.text().len();
		self.editor.reset(
			&mut self.font_system,
			text,
			scene_config_from_runtime(config, layout_width),
		);
		self.invalidate_scene();
		SessionDelta {
			text_changed: true,
			view_changed: true,
			document_edit: Some(TextEdit {
				range: 0..previous_len,
				inserted: text.to_owned(),
			}),
		}
	}

	fn execute_sync_config(&mut self, config: &GlorpConfig, layout_width: f32) -> SessionDelta {
		if !self
			.editor
			.sync_buffer_config(&mut self.font_system, scene_config_from_runtime(config, layout_width))
		{
			return SessionDelta::default();
		}

		self.invalidate_scene();
		SessionDelta {
			view_changed: true,
			..SessionDelta::default()
		}
	}

	fn execute_editor_intent(&mut self, intent: EditorIntent) -> SessionDelta {
		let outcome = self.editor.apply(&mut self.font_system, intent);
		let text_changed = outcome.text_edit.is_some();
		if text_changed {
			self.invalidate_scene();
		}

		SessionDelta {
			text_changed,
			view_changed: false,
			document_edit: outcome.text_edit,
		}
	}

	fn invalidate_scene(&mut self) {
		self.scene = None;
		self.scene_revision = self.scene_revision.saturating_add(1);
	}
}

impl RuntimeState {
	pub fn new(config: GlorpConfig, text: &str) -> Self {
		let session = DocumentSession::new(text, &config, DEFAULT_LAYOUT_WIDTH);

		Self {
			config,
			session,
			revisions: GlorpRevisions { editor: 1, config: 1 },
			perf: PerfProjection::default(),
		}
	}

	pub fn checkpoint(&self) -> RuntimeCheckpoint {
		RuntimeCheckpoint {
			config: self.config.clone(),
			session: self.session.checkpoint(),
			revisions: self.revisions,
			perf: self.perf.clone(),
		}
	}

	pub fn restore(&mut self, checkpoint: RuntimeCheckpoint) {
		self.config = checkpoint.config;
		self.session.restore(checkpoint.session);
		self.revisions = checkpoint.revisions;
		self.perf = checkpoint.perf;
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
	perf: PerfProjection,
}

pub fn text_edit_view(edit: &TextEdit) -> TextEditView {
	TextEditView {
		range: glorp_api::TextRange {
			start: edit.range.start as u64,
			end: edit.range.end as u64,
		},
		inserted: edit.inserted.clone(),
	}
}
