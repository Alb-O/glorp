use {
	super::{CanvasEvent, ControlsMessage, Message, PerfMessage, ShellMessage, SidebarMessage, ViewportMessage},
	crate::{
		canvas::scene_viewport_size,
		panels::{
			CanvasDecorations, CanvasPaneProps, ControlsTabProps, InspectTabProps, SidebarProps, default_sidebar_ratio,
			is_stacked_shell, view_canvas_pane, view_controls_tab, view_inspect_tab, view_perf_tab, view_sidebar,
			view_stacked_shell,
		},
		perf::PerfMonitor,
	},
	glorp_api::{
		ConfigAssignment, EnumValue, GlorpCall, GlorpCallDescriptor, GlorpCaller, GlorpError, GlorpTxn, GlorpValue,
		TextInput, TextRange, WrapChoice,
	},
	glorp_editor::{
		CanvasTarget, EditorEngine, EditorHistoryIntent, EditorIntent, EditorMode, EditorPresentation,
		EditorTextLayerState, EditorViewportMetrics, LayoutRect, OverlayPrimitive, SessionSnapshot, TextEdit,
		make_font_system, sample_preset_text, scene_config,
	},
	glorp_gui::{GuiLaunchOptions, GuiRuntimeClient, GuiRuntimeSession},
	glorp_runtime::{
		GuiEditCommand, GuiEditRequest, GuiEditResponse, GuiRuntimeFrame, GuiSessionHostMessage, GuiSharedDelta,
		SidebarTab,
	},
	iced::{
		Element, Length, Size, Subscription, Theme, Vector,
		widget::{container, pane_grid, responsive},
	},
	std::{
		ops::Range,
		sync::Arc,
		time::{Duration, Instant},
	},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShellPane {
	Sidebar,
	Canvas,
}

struct ShellState {
	active_tab: SidebarTab,
	hovered_target: Option<CanvasTarget>,
	selected_target: Option<CanvasTarget>,
	canvas_focused: bool,
	show_baselines: bool,
	show_hitboxes: bool,
	canvas_scroll: Vector,
	viewport_size: Size,
}

struct InspectSceneState {
	scene: glorp_editor::ScenePresentation,
	layout_width: f32,
	editor_revision: u64,
	config_revision: u64,
}

#[derive(Debug, Clone, Copy)]
struct DocumentSyncState {
	revision: u64,
}

#[derive(Debug, Clone)]
struct LocalEditorContext {
	mode: EditorMode,
	selection: Option<Range<usize>>,
	selection_head: Option<usize>,
}

pub struct RuntimeShell {
	session: GuiRuntimeSession,
	client: GuiRuntimeClient,
	frame: GuiRuntimeFrame,
	layout_width: f32,
	editor: EditorEngine,
	font_system: cosmic_text::FontSystem,
	editor_revision: u64,
	snapshot: SessionSnapshot,
	ui: ShellState,
	inspect_scene: Option<InspectSceneState>,
	scene_refresh_at: Option<Instant>,
	scene_loading: bool,
	document_sync: Option<DocumentSyncState>,
	perf: PerfMonitor,
	shell: pane_grid::State<ShellPane>,
	last_error: Option<String>,
}

impl RuntimeShell {
	pub(crate) fn boot(options: GuiLaunchOptions) -> Self {
		let (session, mut client) =
			GuiRuntimeSession::connect_or_start(options).expect("GUI runtime should connect or start");
		let frame = client.gui_frame().expect("GUI frame should load during boot");
		let layout_width = glorp_runtime::DEFAULT_LAYOUT_WIDTH;
		let ui = ShellState::new(layout_width);
		let mut font_system = make_font_system();
		let editor = EditorEngine::new(
			&mut font_system,
			frame
				.document_text
				.as_deref()
				.expect("GUI frame should be hydrated during boot"),
			shell_scene_config(&frame.config, layout_width),
		);
		let snapshot = build_snapshot(&editor, None, 1, frame.undo_depth, frame.redo_depth);
		Self {
			session,
			client,
			frame,
			layout_width,
			editor,
			font_system,
			editor_revision: 1,
			snapshot,
			ui,
			inspect_scene: None,
			scene_refresh_at: None,
			scene_loading: false,
			document_sync: None,
			perf: PerfMonitor::default(),
			shell: pane_grid::State::with_configuration(pane_grid::Configuration::Split {
				axis: pane_grid::Axis::Vertical,
				ratio: default_sidebar_ratio(),
				a: Box::new(pane_grid::Configuration::Pane(ShellPane::Sidebar)),
				b: Box::new(pane_grid::Configuration::Pane(ShellPane::Canvas)),
			}),
			last_error: None,
		}
	}

	pub(crate) fn title(&self) -> String {
		format!("glorp editor [{}]", self.session.socket_path().display())
	}

	pub(crate) fn update(&mut self, message: Message) {
		self.last_error = self.handle_message(message).err().map(|error| error.to_string());
	}

	fn handle_message(&mut self, message: Message) -> Result<(), GlorpError> {
		match message {
			Message::Controls(message) => self.handle_controls(message),
			Message::Sidebar(SidebarMessage::SelectTab(tab)) => self.select_tab(tab),
			Message::Canvas(message) => self.handle_canvas(message),
			Message::Editor(intent) => self.handle_editor(intent),
			Message::Perf(PerfMessage::Tick(_)) => self.handle_tick(),
			Message::Viewport(ViewportMessage::CanvasResized(size)) => {
				let viewport = scene_viewport_size(size);
				self.resize_viewport(viewport)
			}
			Message::Shell(ShellMessage::PaneResized(event)) => self.resize_shell(event),
		}
	}

	pub(crate) fn subscription(_: &Self) -> Subscription<Message> {
		iced::time::every(std::time::Duration::from_millis(16)).map(|now| Message::Perf(PerfMessage::Tick(now)))
	}

	pub(crate) const fn theme(_: &Self) -> Theme {
		Theme::TokyoNightStorm
	}

	pub(crate) fn view(&self) -> Element<'_, Message> {
		responsive(move |size| {
			let snapshot = Arc::new(self.snapshot.clone());
			if is_stacked_shell(size) {
				view_stacked_shell(
					self.view_sidebar(snapshot.as_ref(), true),
					self.view_canvas(Arc::clone(&snapshot), true),
				)
			} else {
				let grid = pane_grid(&self.shell, move |_, pane, _| {
					let content = match pane {
						ShellPane::Sidebar => self.view_sidebar(snapshot.as_ref(), false),
						ShellPane::Canvas => self.view_canvas(Arc::clone(&snapshot), false),
					};
					pane_grid::Content::new(content)
				})
				.width(Length::Fill)
				.height(Length::Fill)
				.spacing(12)
				.min_size(220)
				.on_resize(12, |event| Message::Shell(ShellMessage::PaneResized(event)));

				container(grid)
					.padding(16)
					.width(Length::Fill)
					.height(Length::Fill)
					.into()
			}
		})
		.width(Length::Fill)
		.height(Length::Fill)
		.into()
	}

	fn view_canvas(&self, snapshot: Arc<SessionSnapshot>, stacked: bool) -> Element<'static, Message> {
		let inspect_targets_active = self.ui.active_tab == SidebarTab::Inspect;
		let inspect_overlays = if inspect_targets_active {
			snapshot.scene.as_ref().map_or_else(
				|| Arc::<[OverlayPrimitive]>::from([]),
				|scene| {
					scene.layout.inspect_overlay_primitives(
						self.ui.hovered_target,
						self.ui.selected_target,
						self.layout_width,
						self.ui.show_hitboxes,
					)
				},
			)
		} else {
			Arc::<[OverlayPrimitive]>::from([])
		};

		view_canvas_pane(CanvasPaneProps {
			snapshot,
			layout_width: self.layout_width,
			decorations: CanvasDecorations {
				show_baselines: self.ui.show_baselines,
				show_hitboxes: self.ui.show_hitboxes,
			},
			inspect_overlays,
			inspect_targets_active,
			focused: self.ui.canvas_focused,
			scroll: self.ui.canvas_scroll,
			perf: self.perf.sink(),
			stacked,
		})
	}

	fn view_sidebar(&self, snapshot: &SessionSnapshot, stacked: bool) -> Element<'static, Message> {
		let body = match self.ui.active_tab {
			SidebarTab::Controls => view_controls_tab(ControlsTabProps {
				preset: self
					.frame
					.config
					.editor
					.preset
					.unwrap_or(glorp_api::SamplePreset::Custom),
				font: self.frame.config.editor.font,
				shaping: self.frame.config.editor.shaping,
				wrapping: self.frame.config.editor.wrapping,
				font_size: self.frame.config.editor.font_size,
				line_height: self.frame.config.editor.line_height,
				show_baselines: self.ui.show_baselines,
				show_hitboxes: self.ui.show_hitboxes,
			}),
			SidebarTab::Inspect => {
				let (warnings, interaction_details) = snapshot.scene.as_ref().map_or_else(
					|| (Arc::<[String]>::from([]), self.inspect_status_text()),
					|scene| {
						let target = self.ui.selected_target.or(self.ui.hovered_target);
						(
							scene.layout.warnings.clone(),
							scene
								.layout
								.target_details(target)
								.unwrap_or_else(|| Arc::<str>::from("hover a run or cluster for details")),
						)
					},
				);
				view_inspect_tab(&InspectTabProps {
					warnings,
					interaction_details,
				})
			}
			SidebarTab::Perf => {
				let dashboard = self.perf.dashboard(
					Some(self.scene_revision_key()),
					snapshot.mode(),
					self.editor.text(),
					snapshot.editor.viewport_metrics,
					self.layout_width,
				);
				view_perf_tab(&dashboard)
			}
		};

		view_sidebar(SidebarProps {
			active_tab: self.ui.active_tab,
			editor_mode: snapshot.mode(),
			editor_bytes: snapshot.editor_bytes(),
			undo_depth: self.frame.undo_depth,
			redo_depth: self.frame.redo_depth,
			body,
			stacked,
		})
	}

	fn handle_controls(&mut self, message: ControlsMessage) -> Result<(), GlorpError> {
		match message {
			ControlsMessage::LoadPreset(preset) => {
				let calls = [
					Some(config_set("editor.preset", enum_value(preset))),
					(preset != glorp_api::SamplePreset::Custom)
						.then(|| document_replace(sample_preset_text(preset).to_owned())),
				]
				.into_iter()
				.flatten()
				.collect();
				self.execute(public_call::<glorp_api::calls::Txn>(GlorpTxn { calls }))?;
				if preset != glorp_api::SamplePreset::Custom {
					self.ui.canvas_scroll = Vector::new(0.0, 0.0);
				}
				Ok(())
			}
			ControlsMessage::FontSelected(font) => self.execute(config_set("editor.font", enum_value(font))),
			ControlsMessage::ShapingSelected(shaping) => {
				self.execute(config_set("editor.shaping", enum_value(shaping)))
			}
			ControlsMessage::WrappingSelected(wrapping) => {
				self.execute(config_set("editor.wrapping", enum_value(wrapping)))
			}
			ControlsMessage::FontSizeChanged(font_size) => self.execute(config_set(
				"editor.font_size",
				GlorpValue::from(serde_json::json!(font_size)),
			)),
			ControlsMessage::LineHeightChanged(line_height) => self.execute(config_set(
				"editor.line_height",
				GlorpValue::from(serde_json::json!(line_height)),
			)),
			ControlsMessage::ShowBaselinesChanged(show_baselines) => self.set_show_baselines(show_baselines),
			ControlsMessage::ShowHitboxesChanged(show_hitboxes) => self.set_show_hitboxes(show_hitboxes),
		}
	}

	fn handle_canvas(&mut self, message: CanvasEvent) -> Result<(), GlorpError> {
		match message {
			CanvasEvent::Hovered(target) => {
				self.ui.hovered_target = inspect_target(self.ui.active_tab, target);
				Ok(())
			}
			CanvasEvent::FocusChanged(focused) => {
				self.ui.canvas_focused = focused;
				Ok(())
			}
			CanvasEvent::ScrollChanged(scroll) => {
				self.ui.canvas_scroll = scroll;
				Ok(())
			}
			CanvasEvent::PointerSelectionStarted { target, intent } => {
				self.ui.canvas_focused = true;
				self.ui.selected_target = inspect_target(self.ui.active_tab, target);
				self.apply_local_editor_intent(EditorIntent::Pointer(intent));
				Ok(())
			}
		}
	}

	fn handle_editor(&mut self, intent: EditorIntent) -> Result<(), GlorpError> {
		match intent {
			EditorIntent::Pointer(pointer) => {
				self.apply_local_editor_intent(EditorIntent::Pointer(pointer));
				Ok(())
			}
			EditorIntent::Motion(motion) => {
				self.apply_local_editor_intent(EditorIntent::Motion(motion));
				Ok(())
			}
			EditorIntent::Mode(mode) => {
				self.apply_local_editor_intent(EditorIntent::Mode(mode));
				Ok(())
			}
			EditorIntent::Edit(edit) => {
				if self.document_sync.is_some() {
					return Ok(());
				}
				self.execute_local_text_edit(EditorIntent::Edit(edit))
			}
			EditorIntent::History(history) => {
				if self.document_sync.is_some() {
					return Ok(());
				}
				self.execute_history_edit(match history {
					EditorHistoryIntent::Undo => GuiEditCommand::Undo,
					EditorHistoryIntent::Redo => GuiEditCommand::Redo,
				})
			}
		}
	}

	fn apply_local_editor_intent(&mut self, intent: EditorIntent) {
		let _ = self.editor.apply(&mut self.font_system, intent);
		self.refresh_local_snapshot();
		if let Some(scroll) = reveal_scroll(&self.snapshot, self.layout_width, &self.ui) {
			self.ui.canvas_scroll = scroll;
		}
	}

	fn execute(&mut self, call: GlorpCall) -> Result<(), GlorpError> {
		self.client.call(call)?;
		Ok(())
	}

	fn execute_local_text_edit(&mut self, intent: EditorIntent) -> Result<(), GlorpError> {
		let base_revision = self.frame.revisions.editor;
		let command_started = Instant::now();
		let outcome = self.editor.apply(&mut self.font_system, intent);
		self.perf.record_editor_command(command_started.elapsed());
		let Some(edit) = outcome.text_edit else {
			self.refresh_local_snapshot();
			if let Some(scroll) = reveal_scroll(&self.snapshot, self.layout_width, &self.ui) {
				self.ui.canvas_scroll = scroll;
			}
			return Ok(());
		};

		self.refresh_local_snapshot();
		if let Some(scroll) = reveal_scroll(&self.snapshot, self.layout_width, &self.ui) {
			self.ui.canvas_scroll = scroll;
		}

		match self.client.gui_edit(GuiEditRequest {
			base_revision,
			command: GuiEditCommand::ReplaceRange {
				range: TextRange {
					start: edit.range.start as u64,
					end: edit.range.end as u64,
				},
				inserted: edit.inserted,
			},
		})? {
			GuiEditResponse::Applied {
				revisions,
				undo_depth,
				redo_depth,
				..
			} => {
				self.frame.revisions = revisions;
				self.frame.undo_depth = undo_depth;
				self.frame.redo_depth = redo_depth;
				self.request_scene_refresh(Duration::from_millis(120));
				Ok(())
			}
			GuiEditResponse::RejectedStale {
				latest_revision,
				undo_depth,
				redo_depth,
			} => {
				self.frame.undo_depth = undo_depth;
				self.frame.redo_depth = redo_depth;
				self.resync_document_to_revision(latest_revision)
			}
		}
	}

	fn execute_history_edit(&mut self, command: GuiEditCommand) -> Result<(), GlorpError> {
		let base_revision = self.frame.revisions.editor;
		match self.client.gui_edit(GuiEditRequest { base_revision, command })? {
			GuiEditResponse::Applied {
				outcome,
				revisions,
				undo_depth,
				redo_depth,
			} => {
				if let Some(edit) = outcome.document_edit {
					self.apply_external_text_edit(TextEdit {
						range: edit.range.start as usize..edit.range.end as usize,
						inserted: edit.inserted,
					});
				} else if outcome.delta.text_changed {
					self.frame.revisions = revisions;
					self.frame.undo_depth = undo_depth;
					self.frame.redo_depth = redo_depth;
					return self.resync_document_to_revision(revisions.editor);
				}
				self.frame.revisions = revisions;
				self.frame.undo_depth = undo_depth;
				self.frame.redo_depth = redo_depth;
				self.request_scene_refresh(Duration::from_millis(120));
				self.refresh_local_snapshot();
				Ok(())
			}
			GuiEditResponse::RejectedStale {
				latest_revision,
				undo_depth,
				redo_depth,
			} => {
				self.frame.undo_depth = undo_depth;
				self.frame.redo_depth = redo_depth;
				self.resync_document_to_revision(latest_revision)
			}
		}
	}

	fn apply_external_text_edit(&mut self, text_edit: TextEdit) {
		let apply_started = Instant::now();
		let _ = self.editor.apply_external_text_edit(&mut self.font_system, text_edit);
		self.perf.record_editor_apply(apply_started.elapsed());
	}

	fn local_context(&self) -> LocalEditorContext {
		let view = self.editor.view_state();
		LocalEditorContext {
			mode: view.mode,
			selection: view.selection,
			selection_head: view.selection_head,
		}
	}

	fn restore_local_context(&mut self, context: &LocalEditorContext) {
		let layout = self.editor.document_layout();
		self.editor
			.replace_context(&layout, context.mode, context.selection.clone(), context.selection_head);
		self.refresh_local_snapshot();
	}

	fn refresh_local_snapshot(&mut self) {
		self.editor_revision += 1;
		self.snapshot = build_snapshot(
			&self.editor,
			self.active_scene(),
			self.editor_revision,
			self.frame.undo_depth,
			self.frame.redo_depth,
		);
	}

	fn resize_viewport(&mut self, viewport: Size) -> Result<(), GlorpError> {
		self.ui.viewport_size = viewport;
		if (self.layout_width - viewport.width).abs() <= f32::EPSILON {
			return Ok(());
		}
		self.layout_width = viewport.width;
		let started = Instant::now();
		let _ = self.editor.sync_buffer_width(&mut self.font_system, viewport.width);
		self.perf.record_editor_width_sync(started.elapsed());
		self.refresh_local_snapshot();
		self.request_scene_refresh(Duration::from_millis(120));
		Ok(())
	}

	fn resize_shell(&mut self, event: iced::widget::pane_grid::ResizeEvent) -> Result<(), GlorpError> {
		self.shell.resize(event.split, event.ratio);
		Ok(())
	}

	fn select_tab(&mut self, tab: SidebarTab) -> Result<(), GlorpError> {
		if self.ui.active_tab == tab {
			return Ok(());
		}
		self.ui.active_tab = tab;
		self.refresh_local_snapshot();
		self.request_scene_refresh(Duration::ZERO);
		Ok(())
	}

	fn set_show_baselines(&mut self, show_baselines: bool) -> Result<(), GlorpError> {
		if self.ui.show_baselines == show_baselines {
			return Ok(());
		}
		self.ui.show_baselines = show_baselines;
		self.refresh_local_snapshot();
		self.request_scene_refresh(Duration::ZERO);
		Ok(())
	}

	fn set_show_hitboxes(&mut self, show_hitboxes: bool) -> Result<(), GlorpError> {
		if self.ui.show_hitboxes == show_hitboxes {
			return Ok(());
		}
		self.ui.show_hitboxes = show_hitboxes;
		self.refresh_local_snapshot();
		self.request_scene_refresh(Duration::ZERO);
		Ok(())
	}

	fn scene_consumer_active(&self) -> bool {
		matches!(self.ui.active_tab, SidebarTab::Inspect) || self.ui.show_baselines || self.ui.show_hitboxes
	}

	fn handle_tick(&mut self) -> Result<(), GlorpError> {
		self.perf.flush_canvas_metrics();
		for event in self.client.drain_events() {
			match event {
				GuiSessionHostMessage::Changed(delta) => self.apply_shared_delta(delta)?,
				GuiSessionHostMessage::Closed => {
					return Err(GlorpError::transport("GUI session closed"));
				}
				GuiSessionHostMessage::Ready { .. } | GuiSessionHostMessage::Reply { .. } => {}
			}
		}
		self.maybe_sync_document()?;
		self.maybe_fetch_scene()?;
		Ok(())
	}

	fn apply_shared_delta(&mut self, delta: GuiSharedDelta) -> Result<(), GlorpError> {
		let revisions = delta.outcome.revisions;
		if revisions.editor <= self.frame.revisions.editor && revisions.config <= self.frame.revisions.config {
			return Ok(());
		}

		if let Some(config) = delta.config {
			self.frame.config = config;
			let _ = self.editor.sync_buffer_config(
				&mut self.font_system,
				shell_scene_config(&self.frame.config, self.layout_width),
			);
		}
		if let Some(edit) = delta.outcome.document_edit.as_ref() {
			self.apply_external_text_edit(TextEdit {
				range: edit.range.start as usize..edit.range.end as usize,
				inserted: edit.inserted.clone(),
			});
		}
		self.frame.revisions = revisions;
		self.frame.undo_depth = delta.undo_depth;
		self.frame.redo_depth = delta.redo_depth;
		if let Some(document_sync) = delta.document_sync {
			self.document_sync = Some(DocumentSyncState {
				revision: document_sync.revision,
			});
		}
		self.request_scene_refresh(Duration::from_millis(120));
		self.refresh_local_snapshot();
		Ok(())
	}

	fn active_scene(&self) -> Option<glorp_editor::ScenePresentation> {
		let scene = self.inspect_scene.as_ref()?;
		(self.scene_consumer_active()
			&& (scene.layout_width - self.layout_width).abs() <= f32::EPSILON
			&& scene.editor_revision == self.frame.revisions.editor
			&& scene.config_revision == self.frame.revisions.config)
			.then(|| scene.scene.clone())
	}

	fn inspect_status_text(&self) -> Arc<str> {
		if self.scene_loading {
			return Arc::<str>::from("scene loading");
		}
		if self.inspect_scene.is_some() {
			return Arc::<str>::from("scene stale");
		}
		Arc::<str>::from("scene unavailable")
	}

	fn request_scene_refresh(&mut self, delay: Duration) {
		if !self.scene_consumer_active() {
			self.scene_refresh_at = None;
			self.scene_loading = false;
			return;
		}

		let deadline = Instant::now() + delay;
		self.scene_refresh_at = Some(self.scene_refresh_at.map_or(deadline, |current| current.min(deadline)));
	}

	fn maybe_fetch_scene(&mut self) -> Result<(), GlorpError> {
		if !self.scene_consumer_active() {
			self.scene_refresh_at = None;
			self.scene_loading = false;
			return Ok(());
		}
		if self.active_scene().is_some() {
			self.scene_refresh_at = None;
			self.scene_loading = false;
			return Ok(());
		}
		if self.scene_loading {
			return Ok(());
		}
		let Some(deadline) = self.scene_refresh_at else {
			return Ok(());
		};
		if Instant::now() < deadline {
			return Ok(());
		}

		self.scene_loading = true;
		self.refresh_local_snapshot();
		let started = Instant::now();
		let scene =
			glorp_editor::ScenePresentation::new(self.scene_revision_key(), self.editor.shared_document_layout());
		self.perf.record_scene_build(started.elapsed());
		self.inspect_scene = Some(InspectSceneState {
			scene,
			layout_width: self.layout_width,
			editor_revision: self.frame.revisions.editor,
			config_revision: self.frame.revisions.config,
		});
		self.scene_loading = false;
		self.scene_refresh_at = None;
		self.refresh_local_snapshot();
		Ok(())
	}

	fn maybe_sync_document(&mut self) -> Result<(), GlorpError> {
		let Some(document_sync) = self.document_sync else {
			return Ok(());
		};
		self.resync_document_to_revision(document_sync.revision)
	}

	fn resync_document_to_revision(&mut self, revision: u64) -> Result<(), GlorpError> {
		let (response, bytes) = self.client.document_fetch(revision)?;
		if response.revision < revision {
			return Err(GlorpError::transport(format!(
				"document fetch returned editor revision `{}` below requested revision `{revision}`",
				response.revision
			)));
		}

		let text = String::from_utf8(bytes)
			.map_err(|error| GlorpError::transport(format!("document payload is not valid UTF-8: {error}")))?;
		let context = clamp_local_context(self.local_context(), text.len());
		self.apply_external_text_edit(TextEdit {
			range: 0..self.editor.text().len(),
			inserted: text,
		});
		self.frame.revisions.editor = response.revision;
		self.document_sync = None;
		self.restore_local_context(&context);
		self.request_scene_refresh(Duration::from_millis(120));
		Ok(())
	}

	fn scene_revision_key(&self) -> u64 {
		self.frame.revisions.editor.max(self.frame.revisions.config)
	}
}

impl ShellState {
	fn new(layout_width: f32) -> Self {
		Self {
			active_tab: SidebarTab::Controls,
			hovered_target: None,
			selected_target: None,
			canvas_focused: false,
			show_baselines: false,
			show_hitboxes: false,
			canvas_scroll: Vector::new(0.0, 0.0),
			viewport_size: Size::new(layout_width, 320.0),
		}
	}
}

fn shell_scene_config(config: &glorp_api::GlorpConfig, layout_width: f32) -> glorp_editor::SceneConfig {
	scene_config(
		config.editor.font,
		config.editor.shaping,
		config.editor.wrapping,
		config.editor.font_size,
		config.editor.line_height,
		layout_width,
	)
}

fn build_snapshot(
	editor: &EditorEngine, scene: Option<glorp_editor::ScenePresentation>, revision: u64, undo_depth: usize,
	redo_depth: usize,
) -> SessionSnapshot {
	let viewport_metrics = editor.viewport_metrics();
	let text_layer = editor.text_layer_state();
	let editor = EditorPresentation::new(
		revision,
		viewport_metrics,
		EditorTextLayerState {
			buffer: text_layer.buffer,
			measured_height: text_layer.measured_height,
		},
		editor.view_state(),
		editor.text().len(),
		undo_depth,
		redo_depth,
	);
	SessionSnapshot { editor, scene }
}

fn config_set(path: &str, value: GlorpValue) -> GlorpCall {
	public_call::<glorp_api::calls::ConfigSet>(ConfigAssignment {
		path: path.to_owned(),
		value,
	})
}

fn document_replace(text: impl Into<String>) -> GlorpCall {
	public_call::<glorp_api::calls::DocumentReplace>(TextInput { text: text.into() })
}

fn enum_value<T>(value: T) -> GlorpValue
where
	T: EnumValue, {
	value.as_ref().into()
}

fn inspect_target(active_tab: SidebarTab, target: Option<CanvasTarget>) -> Option<CanvasTarget> {
	target.filter(|_| active_tab == SidebarTab::Inspect)
}

fn reveal_scroll(snapshot: &SessionSnapshot, layout_width: f32, ui: &ShellState) -> Option<Vector> {
	let target = snapshot.editor.editor.viewport_target?;
	let metrics = snapshot.editor.viewport_metrics;
	let viewport = Size::new(ui.viewport_size.width.max(1.0), ui.viewport_size.height.max(1.0));
	let current = ui.canvas_scroll;
	let next = reveal_target_scroll(current, target, metrics, layout_width, viewport);
	let delta = next - current;
	(delta.x.abs() > 0.5 || delta.y.abs() > 0.5).then_some(next)
}

fn reveal_target_scroll(
	current: Vector, target: LayoutRect, metrics: EditorViewportMetrics, layout_width: f32, viewport: Size,
) -> Vector {
	let margin_x = 24.0;
	let margin_y = 24.0;
	let mut scroll = clamp_scroll(current, metrics, layout_width, viewport);
	let left = target.x;
	let right = target.x + target.width.max(1.0);
	let top = target.y;
	let bottom = target.y + target.height.max(1.0);

	if left < scroll.x + margin_x {
		scroll.x = (left - margin_x).max(0.0);
	} else if right > scroll.x + viewport.width - margin_x {
		scroll.x = (right - viewport.width + margin_x).max(0.0);
	}

	if top < scroll.y + margin_y {
		scroll.y = (top - margin_y).max(0.0);
	} else if bottom > scroll.y + viewport.height - margin_y {
		scroll.y = (bottom - viewport.height + margin_y).max(0.0);
	}

	clamp_scroll(scroll, metrics, layout_width, viewport)
}

fn clamp_scroll(scroll: Vector, metrics: EditorViewportMetrics, layout_width: f32, viewport: Size) -> Vector {
	let max_x = if matches!(metrics.wrapping, WrapChoice::None) {
		(metrics.measured_width.max(layout_width) - viewport.width).max(0.0)
	} else {
		(layout_width - viewport.width).max(0.0)
	};
	let max_y = (metrics.measured_height - viewport.height).max(0.0);
	Vector::new(scroll.x.clamp(0.0, max_x), scroll.y.clamp(0.0, max_y))
}

fn clamp_local_context(mut context: LocalEditorContext, text_len: usize) -> LocalEditorContext {
	context.selection = context.selection.map(|range| {
		let start = range.start.min(text_len);
		let end = range.end.min(text_len).max(start);
		start..end
	});
	context.selection_head = context.selection_head.map(|head| head.min(text_len));
	context
}

fn public_call<D>(input: D::Input) -> GlorpCall
where
	D: GlorpCallDescriptor, {
	D::build(input).expect("GUI public call should encode")
}
