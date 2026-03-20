use {
	crate::{
		canvas_view::scene_viewport_size,
		editor::{EditorEditIntent, EditorHistoryIntent, EditorIntent, EditorMode, EditorViewportMetrics},
		overlay::OverlayPrimitive,
		perf::{PerfMonitor, unavailable_dashboard},
		presentation::SessionSnapshot,
		types::{CanvasEvent, ControlsMessage, Message, PerfMessage, ViewportMessage},
		ui::{
			CanvasDecorations, CanvasPaneProps, ControlsTabProps, InspectTabProps, SidebarProps, default_sidebar_ratio,
			is_stacked_shell, view_canvas_pane, view_controls_tab, view_inspect_tab, view_perf_tab, view_sidebar,
			view_stacked_shell,
		},
	},
	glorp_api::{
		ConfigAssignment, EditorContextView, EditorHistoryCommand, EnumValue, GlorpCall, GlorpCallDescriptor,
		GlorpCaller, GlorpError, GlorpTxn, GlorpValue, TextInput, WrapChoice,
	},
	glorp_editor::{
		CanvasTarget, EditorEngine, EditorPresentation, EditorTextLayerState, LayoutRect, make_font_system,
		sample_preset_text, scene_config,
	},
	glorp_gui::{GuiLaunchOptions, GuiRuntimeClient, GuiRuntimeSession},
	glorp_runtime::{
		GuiCommand, GuiEditCommand, GuiEditRequest, GuiRuntimeFrame, GuiSessionHostMessage, GuiSharedDelta, SidebarTab,
	},
	iced::{
		Element, Length, Size, Subscription, Theme, Vector,
		widget::{container, pane_grid, responsive},
	},
	std::sync::Arc,
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

pub struct RuntimeShell {
	session: GuiRuntimeSession,
	client: GuiRuntimeClient,
	frame: GuiRuntimeFrame,
	editor: EditorEngine,
	font_system: cosmic_text::FontSystem,
	editor_revision: u64,
	snapshot: SessionSnapshot,
	ui: ShellState,
	perf: PerfMonitor,
	shell: pane_grid::State<ShellPane>,
	last_error: Option<String>,
}

impl RuntimeShell {
	pub(crate) fn boot(options: GuiLaunchOptions) -> Self {
		let (session, mut client) =
			GuiRuntimeSession::connect_or_start(options).expect("GUI runtime should connect or start");
		let frame = client.gui_frame().expect("GUI frame should load during boot");
		let ui = ShellState::new(frame.layout_width);
		let mut font_system = make_font_system();
		let editor = EditorEngine::new(
			&mut font_system,
			frame.document_text.as_str(),
			shell_scene_config(&frame),
		);
		let snapshot = build_snapshot(&editor, &frame, 1);
		Self {
			session,
			client,
			frame,
			editor,
			font_system,
			editor_revision: 1,
			snapshot,
			ui,
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
			Message::Sidebar(crate::types::SidebarMessage::SelectTab(tab)) => self.select_tab(tab),
			Message::Canvas(message) => self.handle_canvas(message),
			Message::Editor(intent) => self.handle_editor(intent),
			Message::Perf(PerfMessage::Tick(_)) => self.handle_tick(),
			Message::Viewport(ViewportMessage::CanvasResized(size)) => {
				let viewport = scene_viewport_size(size);
				self.resize_viewport(viewport)
			}
			Message::Shell(crate::types::ShellMessage::PaneResized(event)) => self.resize_shell(event),
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
				.on_resize(12, |event| {
					Message::Shell(crate::types::ShellMessage::PaneResized(event))
				});

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
						self.frame.layout_width,
						self.ui.show_hitboxes,
					)
				},
			)
		} else {
			Arc::<[OverlayPrimitive]>::from([])
		};

		view_canvas_pane(CanvasPaneProps {
			snapshot,
			layout_width: self.frame.layout_width,
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
					|| (Arc::<[String]>::from([]), Arc::<str>::from("derived scene unavailable")),
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
			SidebarTab::Perf => snapshot.scene.as_ref().map_or_else(
				|| {
					let dashboard =
						unavailable_dashboard(snapshot.mode(), snapshot.editor_bytes(), self.frame.layout_width);
					view_perf_tab(&dashboard)
				},
				|scene| {
					let dashboard = self
						.perf
						.dashboard(&scene.layout, snapshot.mode(), snapshot.editor_bytes());
					view_perf_tab(&dashboard)
				},
			),
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
				self.apply_local_editor_intent(EditorIntent::Pointer(intent))
			}
		}
	}

	fn handle_editor(&mut self, intent: EditorIntent) -> Result<(), GlorpError> {
		match intent {
			EditorIntent::Pointer(pointer) => self.apply_local_editor_intent(EditorIntent::Pointer(pointer)),
			EditorIntent::Motion(motion) => self.apply_local_editor_intent(EditorIntent::Motion(motion)),
			EditorIntent::Mode(mode) => self.apply_local_editor_intent(EditorIntent::Mode(mode)),
			EditorIntent::Edit(edit) => self.execute_gui_edit(EditorIntent::Edit(edit.clone()), edit_command(edit)),
			EditorIntent::History(history) => self.execute_gui_edit(
				EditorIntent::History(history),
				GuiEditCommand::History(match history {
					EditorHistoryIntent::Undo => EditorHistoryCommand::Undo,
					EditorHistoryIntent::Redo => EditorHistoryCommand::Redo,
				}),
			),
		}
	}

	fn apply_local_editor_intent(&mut self, intent: EditorIntent) -> Result<(), GlorpError> {
		let _ = self.editor.apply(&mut self.font_system, intent);
		self.refresh_local_snapshot();
		if let Some(scroll) = reveal_scroll(&self.snapshot, self.frame.layout_width, &self.ui) {
			self.ui.canvas_scroll = scroll;
		}
		Ok(())
	}

	fn execute(&mut self, call: GlorpCall) -> Result<(), GlorpError> {
		self.client.call(call)?;
		Ok(())
	}

	fn execute_gui_edit(&mut self, intent: EditorIntent, command: GuiEditCommand) -> Result<(), GlorpError> {
		let context = self.local_context();
		self.apply_local_editor_intent(intent)?;
		let response = self.client.gui_edit(GuiEditRequest {
			layout: glorp_runtime::GuiLayoutRequest {
				layout_width: self.frame.layout_width,
			},
			context,
			command,
		})?;
		self.frame.revisions = response.revisions;
		self.frame.undo_depth = response.undo_depth;
		self.frame.redo_depth = response.redo_depth;
		if response.outcome.delta.text_changed || response.outcome.delta.config_changed {
			self.frame.scene = None;
		}
		self.apply_local_context(&response.next_context)?;
		if self.scene_required() && self.frame.scene.is_none() {
			self.refresh_scene()?;
		}
		Ok(())
	}

	fn refresh_scene(&mut self) -> Result<(), GlorpError> {
		self.client.execute_gui(GuiCommand::SceneEnsure)?;
		let frame = self.client.gui_frame()?;
		self.frame.scene = frame.scene;
		self.frame.undo_depth = frame.undo_depth;
		self.frame.redo_depth = frame.redo_depth;
		Ok(())
	}

	fn apply_local_context(&mut self, context: &EditorContextView) -> Result<(), GlorpError> {
		let layout = self.editor.document_layout();
		let mode = match context.mode {
			glorp_api::EditorMode::Normal => EditorMode::Normal,
			glorp_api::EditorMode::Insert => EditorMode::Insert,
		};
		let selection = context
			.selection
			.as_ref()
			.map(|range| range.start as usize..range.end as usize);
		let selection_head = context.selection_head.map(|head| head as usize);
		self.editor.replace_context(&layout, mode, selection, selection_head);
		self.refresh_local_snapshot();
		Ok(())
	}

	fn local_context(&self) -> EditorContextView {
		let view = self.editor.view_state();
		EditorContextView {
			mode: match view.mode {
				EditorMode::Normal => glorp_api::EditorMode::Normal,
				EditorMode::Insert => glorp_api::EditorMode::Insert,
			},
			selection: view.selection.map(|range| glorp_api::TextRange {
				start: range.start as u64,
				end: range.end as u64,
			}),
			selection_head: view.selection_head.map(|head| head as u64),
		}
	}

	fn refresh_local_snapshot(&mut self) {
		self.editor_revision += 1;
		self.snapshot = build_snapshot(&self.editor, &self.frame, self.editor_revision);
	}

	fn resize_viewport(&mut self, viewport: Size) -> Result<(), GlorpError> {
		self.ui.viewport_size = viewport;
		if (self.frame.layout_width - viewport.width).abs() <= f32::EPSILON {
			return Ok(());
		}
		self.client.set_layout_width(viewport.width);
		self.frame.layout_width = viewport.width;
		self.frame.scene = None;
		let _ = self
			.editor
			.sync_buffer_config(&mut self.font_system, shell_scene_config(&self.frame));
		self.refresh_local_snapshot();
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
		if self.scene_required() && self.frame.scene.is_none() {
			self.refresh_scene()?;
		}
		Ok(())
	}

	fn set_show_baselines(&mut self, show_baselines: bool) -> Result<(), GlorpError> {
		if self.ui.show_baselines == show_baselines {
			return Ok(());
		}
		self.ui.show_baselines = show_baselines;
		if self.scene_required() && self.frame.scene.is_none() {
			self.refresh_scene()?;
		}
		Ok(())
	}

	fn set_show_hitboxes(&mut self, show_hitboxes: bool) -> Result<(), GlorpError> {
		if self.ui.show_hitboxes == show_hitboxes {
			return Ok(());
		}
		self.ui.show_hitboxes = show_hitboxes;
		if self.scene_required() && self.frame.scene.is_none() {
			self.refresh_scene()?;
		}
		Ok(())
	}

	fn scene_required(&self) -> bool {
		matches!(self.ui.active_tab, SidebarTab::Inspect | SidebarTab::Perf)
			|| self.ui.show_baselines
			|| self.ui.show_hitboxes
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
		Ok(())
	}

	fn apply_shared_delta(&mut self, delta: GuiSharedDelta) -> Result<(), GlorpError> {
		let revisions = delta.outcome.revisions;
		if revisions.editor <= self.frame.revisions.editor && revisions.config <= self.frame.revisions.config {
			return Ok(());
		}

		if let Some(config) = delta.config {
			self.frame.config = config;
			let _ = self
				.editor
				.sync_buffer_config(&mut self.font_system, shell_scene_config(&self.frame));
		}
		if let Some(edit) = delta.outcome.document_edit.as_ref() {
			let _ = self.editor.apply_external_text_edit(
				&mut self.font_system,
				glorp_editor::TextEdit {
					range: edit.range.start as usize..edit.range.end as usize,
					inserted: edit.inserted.clone(),
				},
			);
		}
		self.frame.revisions = revisions;
		self.frame.undo_depth = delta.undo_depth;
		self.frame.redo_depth = delta.redo_depth;
		if delta.outcome.delta.text_changed || delta.outcome.delta.config_changed {
			self.frame.scene = None;
		}
		if self.scene_required() && self.frame.scene.is_none() {
			self.refresh_scene()?;
		}
		self.refresh_local_snapshot();
		Ok(())
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

fn shell_scene_config(frame: &GuiRuntimeFrame) -> glorp_editor::SceneConfig {
	scene_config(
		frame.config.editor.font,
		frame.config.editor.shaping,
		frame.config.editor.wrapping,
		frame.config.editor.font_size,
		frame.config.editor.line_height,
		frame.layout_width,
	)
}

fn build_snapshot(editor: &EditorEngine, frame: &GuiRuntimeFrame, revision: u64) -> SessionSnapshot {
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
		frame.undo_depth,
		frame.redo_depth,
	);
	SessionSnapshot {
		editor,
		scene: frame.scene.clone(),
	}
}

fn edit_command(edit: EditorEditIntent) -> GuiEditCommand {
	match edit {
		EditorEditIntent::Backspace => GuiEditCommand::Backspace,
		EditorEditIntent::DeleteForward => GuiEditCommand::DeleteForward,
		EditorEditIntent::DeleteSelection => GuiEditCommand::DeleteSelection,
		EditorEditIntent::InsertText(text) => GuiEditCommand::InsertText(text),
	}
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

fn public_call<D>(input: D::Input) -> GlorpCall
where
	D: GlorpCallDescriptor, {
	D::build(input).expect("GUI public call should encode")
}
