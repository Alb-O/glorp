use {
	crate::{perf::PerfProjection, scene::scene_config_from_runtime},
	glorp_api::{CanvasTarget, GlorpConfig, GlorpDelta, GlorpRevisions, SidebarTab},
	glorp_editor::{
		EditorEngine, EditorIntent, EditorMode, EditorOutcome, EditorPresentation, EditorViewState, ScenePresentation,
		SessionSnapshot, make_font_system,
	},
	std::{
		ops::Range,
		time::{Duration, Instant},
	},
};

#[derive(Debug, Clone, PartialEq)]
pub enum SessionRequest {
	ReplaceDocument(String),
	SyncConfig,
	ApplyEditorIntent(EditorIntent),
	EnsureScene,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SessionChange {
	Text,
	View,
	Selection,
	Mode,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct SessionChanges(u8);

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SessionDelta {
	changes: SessionChanges,
	pub scene_materialized: Option<Duration>,
}

#[derive(Debug, Clone)]
pub struct SessionFeedback {
	pub delta: SessionDelta,
}

#[derive(Debug, Clone)]
pub struct DocumentCheckpoint {
	editor: EditorEngine,
	snapshot: SessionSnapshot,
	next_scene_revision: u64,
}

#[derive(Debug)]
pub struct DocumentSession {
	editor: EditorEngine,
	snapshot: SessionSnapshot,
	next_scene_revision: u64,
	font_system: cosmic_text::FontSystem,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UiRuntimeState {
	pub active_tab: SidebarTab,
	pub hovered_target: Option<CanvasTarget>,
	pub selected_target: Option<CanvasTarget>,
	pub canvas_focused: bool,
	pub canvas_scroll_x: f32,
	pub canvas_scroll_y: f32,
	pub layout_width: f32,
	pub viewport_width: f32,
	pub viewport_height: f32,
	pub pane_ratio: f32,
}

#[derive(Debug)]
pub struct RuntimeState {
	pub config: GlorpConfig,
	pub session: DocumentSession,
	pub ui: UiRuntimeState,
	pub revisions: GlorpRevisions,
	pub perf: PerfProjection,
}

impl SessionChange {
	const fn bit(self) -> u8 {
		match self {
			Self::Text => 1 << 0,
			Self::View => 1 << 1,
			Self::Selection => 1 << 2,
			Self::Mode => 1 << 3,
		}
	}
}

impl SessionChanges {
	const fn with(mut self, change: SessionChange) -> Self {
		self.0 |= change.bit();
		self
	}

	const fn with_if(self, condition: bool, change: SessionChange) -> Self {
		if condition { self.with(change) } else { self }
	}

	const fn contains(self, change: SessionChange) -> bool {
		self.0 & change.bit() != 0
	}
}

impl SessionDelta {
	fn with_changes(changes: SessionChanges) -> Self {
		Self {
			changes,
			..Self::default()
		}
	}

	pub fn text_changed(&self) -> bool {
		self.changes.contains(SessionChange::Text)
	}

	pub fn view_changed(&self) -> bool {
		self.changes.contains(SessionChange::View)
	}

	pub fn selection_changed(&self) -> bool {
		self.changes.contains(SessionChange::Selection)
	}

	pub fn mode_changed(&self) -> bool {
		self.changes.contains(SessionChange::Mode)
	}
}

impl DocumentSession {
	pub fn new(text: &str, config: &GlorpConfig, layout_width: f32) -> Self {
		let mut font_system = make_font_system();
		let scene_config = scene_config_from_runtime(config, layout_width);
		let editor = EditorEngine::new(&mut font_system, text, scene_config);
		let snapshot = SessionSnapshot::new(build_editor_presentation(&editor, 1));

		Self {
			editor,
			snapshot,
			next_scene_revision: 0,
			font_system,
		}
	}

	pub fn execute(&mut self, request: SessionRequest, config: &GlorpConfig, layout_width: f32) -> SessionFeedback {
		let ensure_scene = matches!(request, SessionRequest::EnsureScene);
		let mut delta = match request {
			SessionRequest::ReplaceDocument(text) => self.execute_replace_document(&text, config, layout_width),
			SessionRequest::SyncConfig => self.execute_sync_config(config, layout_width),
			SessionRequest::ApplyEditorIntent(intent) => self.execute_editor_intent(intent),
			SessionRequest::EnsureScene => SessionDelta::default(),
		};

		if ensure_scene {
			delta.scene_materialized = self.materialize_scene_if_needed();
		}

		SessionFeedback { delta }
	}

	pub fn text(&self) -> &str {
		self.editor.text()
	}

	pub fn snapshot(&self) -> &SessionSnapshot {
		&self.snapshot
	}

	pub fn checkpoint(&self) -> DocumentCheckpoint {
		DocumentCheckpoint {
			editor: self.editor.clone(),
			snapshot: self.snapshot.clone(),
			next_scene_revision: self.next_scene_revision,
		}
	}

	pub fn restore(&mut self, checkpoint: DocumentCheckpoint) {
		self.editor = checkpoint.editor;
		self.snapshot = checkpoint.snapshot;
		self.next_scene_revision = checkpoint.next_scene_revision;
		self.font_system = make_font_system();
	}

	fn execute_replace_document(&mut self, text: &str, config: &GlorpConfig, layout_width: f32) -> SessionDelta {
		self.editor.reset(
			&mut self.font_system,
			text,
			scene_config_from_runtime(config, layout_width),
		);
		self.refresh_editor_snapshot();
		self.invalidate_scene();
		SessionDelta::with_changes(
			SessionChanges::default()
				.with(SessionChange::Text)
				.with(SessionChange::View)
				.with(SessionChange::Selection)
				.with(SessionChange::Mode),
		)
	}

	fn execute_sync_config(&mut self, config: &GlorpConfig, layout_width: f32) -> SessionDelta {
		if !self
			.editor
			.sync_buffer_config(&mut self.font_system, scene_config_from_runtime(config, layout_width))
		{
			return SessionDelta::default();
		}

		self.refresh_editor_snapshot();
		self.invalidate_scene();
		SessionDelta::with_changes(SessionChanges::default().with(SessionChange::View))
	}

	fn execute_editor_intent(&mut self, intent: EditorIntent) -> SessionDelta {
		let EditorOutcome {
			view_changed,
			selection_changed,
			mode_changed,
			viewport_target: _,
			text_edit,
		} = self.editor.apply(&mut self.font_system, intent);
		let text_changed = text_edit.is_some();

		if view_changed || selection_changed || mode_changed || text_changed {
			self.refresh_editor_snapshot();
		}

		if text_changed {
			self.invalidate_scene();
		}

		SessionDelta::with_changes(
			SessionChanges::default()
				.with_if(text_changed, SessionChange::Text)
				.with_if(view_changed, SessionChange::View)
				.with_if(selection_changed, SessionChange::Selection)
				.with_if(mode_changed, SessionChange::Mode),
		)
	}

	fn refresh_editor_snapshot(&mut self) {
		let revision = self.snapshot.editor.revision + 1;
		self.snapshot.editor = build_editor_presentation(&self.editor, revision);
	}

	fn invalidate_scene(&mut self) {
		self.snapshot.scene = None;
	}

	fn materialize_scene_if_needed(&mut self) -> Option<Duration> {
		if self.snapshot.scene.is_some() {
			return None;
		}

		let revision = self.next_scene_revision + 1;
		let started = Instant::now();
		self.snapshot.scene = Some(ScenePresentation::new(revision, self.editor.shared_document_layout()));
		self.next_scene_revision = revision;
		Some(started.elapsed())
	}
}

impl UiRuntimeState {
	pub fn new() -> Self {
		Self {
			active_tab: SidebarTab::Controls,
			hovered_target: None,
			selected_target: None,
			canvas_focused: false,
			canvas_scroll_x: 0.0,
			canvas_scroll_y: 0.0,
			layout_width: 540.0,
			viewport_width: 540.0,
			viewport_height: 320.0,
			pane_ratio: 0.35,
		}
	}
}

impl RuntimeState {
	pub fn new(config: GlorpConfig, text: String) -> Self {
		let ui = UiRuntimeState::new();
		let session = DocumentSession::new(&text, &config, ui.layout_width);

		Self {
			config,
			session,
			ui,
			revisions: GlorpRevisions {
				editor: 1,
				scene: None,
				config: 1,
			},
			perf: PerfProjection::default(),
		}
	}

	pub fn checkpoint(&self) -> RuntimeCheckpoint {
		RuntimeCheckpoint {
			config: self.config.clone(),
			session: self.session.checkpoint(),
			ui: self.ui.clone(),
			revisions: self.revisions,
			perf: self.perf.clone(),
		}
	}

	pub fn restore(&mut self, checkpoint: RuntimeCheckpoint) {
		self.config = checkpoint.config;
		self.session.restore(checkpoint.session);
		self.ui = checkpoint.ui;
		self.revisions = checkpoint.revisions;
		self.perf = checkpoint.perf;
	}

	pub fn delta_from_session(&mut self, session_delta: &SessionDelta) -> GlorpDelta {
		let scene_changed = session_delta.scene_materialized.is_some();
		if session_delta.text_changed()
			|| session_delta.view_changed()
			|| session_delta.selection_changed()
			|| session_delta.mode_changed()
		{
			self.revisions.editor += 1;
		}

		if let Some(scene) = self.session.snapshot().scene.as_ref() {
			self.revisions.scene = Some(scene.revision);
		} else if session_delta.text_changed() || session_delta.view_changed() {
			self.revisions.scene = None;
		}

		if let Some(duration) = session_delta.scene_materialized {
			self.perf.record_scene_build(duration.as_secs_f64() * 1000.0);
		}

		GlorpDelta {
			text_changed: session_delta.text_changed(),
			view_changed: session_delta.view_changed(),
			selection_changed: session_delta.selection_changed(),
			mode_changed: session_delta.mode_changed(),
			config_changed: false,
			ui_changed: false,
			scene_changed,
		}
	}
}

#[derive(Debug, Clone)]
pub struct RuntimeCheckpoint {
	config: GlorpConfig,
	session: DocumentCheckpoint,
	ui: UiRuntimeState,
	revisions: GlorpRevisions,
	perf: PerfProjection,
}

fn build_editor_presentation(editor: &EditorEngine, revision: u64) -> EditorPresentation {
	let (undo_depth, redo_depth) = editor.history_depths();
	EditorPresentation::new(
		revision,
		editor.viewport_metrics(),
		editor.text_layer_state(),
		editor.view_state(),
		editor.text().len(),
		undo_depth,
		redo_depth,
	)
}

pub fn selection_range(selection: Option<Range<usize>>) -> Option<glorp_api::TextRange> {
	selection.map(|selection| glorp_api::TextRange {
		start: selection.start as u64,
		end: selection.end as u64,
	})
}

pub fn selection_head(view: &EditorViewState) -> Option<u64> {
	view.selection_head.map(|head| head as u64)
}

pub fn pointer_anchor(view: &EditorViewState) -> Option<u64> {
	view.pointer_anchor.map(|anchor| anchor as u64)
}

pub fn mode(mode: EditorMode) -> glorp_api::EditorMode {
	match mode {
		EditorMode::Normal => glorp_api::EditorMode::Normal,
		EditorMode::Insert => glorp_api::EditorMode::Insert,
	}
}
