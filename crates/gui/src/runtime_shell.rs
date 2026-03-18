use {
	crate::{
		canvas_view::scene_viewport_size,
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
		ConfigAssignment, EditorHistoryInput, EditorModeInput, EditorMotionInput, EnumValue, GlorpError, GlorpExec,
		GlorpHost, GlorpTxn, GlorpValue, TextInput, WrapChoice,
	},
	glorp_editor::{
		CanvasTarget, EditorEditIntent, EditorHistoryIntent, EditorIntent, EditorModeIntent, EditorMotion,
		EditorPointerIntent, EditorViewportMetrics, LayoutRect, sample_preset_text,
	},
	glorp_gui::{GuiLaunchOptions, GuiRuntimeSession},
	glorp_runtime::{GuiCommand, GuiRuntimeFrame, RuntimeHost, SidebarTab},
	iced::{
		Element, Length, Size, Subscription, Theme, Vector,
		widget::{container, pane_grid, responsive},
	},
	std::sync::{Arc, Mutex},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShellPane {
	Sidebar,
	Canvas,
}

pub struct RuntimeShell {
	session: GuiRuntimeSession,
	host: Arc<Mutex<RuntimeHost>>,
	frame: GuiRuntimeFrame,
	perf: PerfMonitor,
	shell: pane_grid::State<ShellPane>,
	last_error: Option<String>,
}

impl RuntimeShell {
	pub(crate) fn boot(options: GuiLaunchOptions) -> Self {
		let (session, _client) = GuiRuntimeSession::start_owned(options).expect("GUI runtime should start");
		let host = session.host();
		let frame = {
			let mut host = host
				.lock()
				.expect("GUI runtime lock should not be poisoned during boot");
			host.gui_frame()
		};
		let mut shell = Self {
			session,
			host,
			frame,
			perf: PerfMonitor::default(),
			shell: pane_grid::State::with_configuration(pane_grid::Configuration::Split {
				axis: pane_grid::Axis::Vertical,
				ratio: default_sidebar_ratio(),
				a: Box::new(pane_grid::Configuration::Pane(ShellPane::Sidebar)),
				b: Box::new(pane_grid::Configuration::Pane(ShellPane::Canvas)),
			}),
			last_error: None,
		};
		let _ = shell.refresh_frame();
		shell
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
			Message::Sidebar(crate::types::SidebarMessage::SelectTab(tab)) => {
				self.execute_gui(GuiCommand::SidebarSelect(tab))
			}
			Message::Canvas(message) => self.handle_canvas(message),
			Message::Editor(intent) => self.handle_editor(intent),
			Message::Perf(PerfMessage::Tick(_)) => self.refresh_frame(),
			Message::Viewport(ViewportMessage::CanvasResized(size)) => {
				let viewport = scene_viewport_size(size);
				self.execute_gui(GuiCommand::ViewportMetricsSet {
					layout_width: viewport.width,
					viewport_width: viewport.width,
					viewport_height: viewport.height,
				})
			}
			Message::Shell(crate::types::ShellMessage::PaneResized(event)) => {
				self.shell.resize(event.split, event.ratio);
				self.execute_gui(GuiCommand::PaneRatioSet(event.ratio))
			}
		}
	}

	pub(crate) fn subscription(_: &Self) -> Subscription<Message> {
		iced::time::every(std::time::Duration::from_millis(100)).map(|now| Message::Perf(PerfMessage::Tick(now)))
	}

	pub(crate) const fn theme(_: &Self) -> Theme {
		Theme::TokyoNightStorm
	}

	pub(crate) fn view(&self) -> Element<'_, Message> {
		responsive(move |size| {
			let snapshot = Arc::new(self.frame.snapshot.clone());
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
		let inspect_targets_active = self.frame.ui.active_tab == SidebarTab::Inspect;
		let inspect_overlays = if inspect_targets_active {
			snapshot.scene.as_ref().map_or_else(
				|| Arc::<[OverlayPrimitive]>::from([]),
				|scene| {
					scene.layout.inspect_overlay_primitives(
						self.frame.ui.hovered_target,
						self.frame.ui.selected_target,
						self.frame.ui.layout_width,
						self.frame.ui.show_hitboxes,
					)
				},
			)
		} else {
			Arc::<[OverlayPrimitive]>::from([])
		};

		view_canvas_pane(CanvasPaneProps {
			snapshot,
			layout_width: self.frame.ui.layout_width,
			decorations: CanvasDecorations {
				show_baselines: self.frame.ui.show_baselines,
				show_hitboxes: self.frame.ui.show_hitboxes,
			},
			inspect_overlays,
			inspect_targets_active,
			focused: self.frame.ui.canvas_focused,
			scroll: Vector::new(self.frame.ui.canvas_scroll_x, self.frame.ui.canvas_scroll_y),
			perf: self.perf.sink(),
			stacked,
		})
	}

	fn view_sidebar(&self, snapshot: &SessionSnapshot, stacked: bool) -> Element<'static, Message> {
		let body = match self.frame.ui.active_tab {
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
				show_baselines: self.frame.ui.show_baselines,
				show_hitboxes: self.frame.ui.show_hitboxes,
			}),
			SidebarTab::Inspect => {
				let (warnings, interaction_details) = snapshot.scene.as_ref().map_or_else(
					|| (Arc::<[String]>::from([]), Arc::<str>::from("derived scene unavailable")),
					|scene| {
						let target = self.frame.ui.selected_target.or(self.frame.ui.hovered_target);
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
						unavailable_dashboard(snapshot.mode(), snapshot.editor_bytes(), self.frame.ui.layout_width);
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
			active_tab: self.frame.ui.active_tab,
			editor_mode: snapshot.mode(),
			editor_bytes: snapshot.editor_bytes(),
			undo_depth: snapshot.editor.undo_depth,
			redo_depth: snapshot.editor.redo_depth,
			body,
			stacked,
		})
	}

	fn handle_controls(&mut self, message: ControlsMessage) -> Result<(), GlorpError> {
		match message {
			ControlsMessage::LoadPreset(preset) => {
				let execs = [
					Some(config_set("editor.preset", enum_value(preset))),
					(preset != glorp_api::SamplePreset::Custom).then(|| {
						GlorpExec::DocumentReplace(TextInput {
							text: sample_preset_text(preset).to_owned(),
						})
					}),
				]
				.into_iter()
				.flatten()
				.collect();
				self.execute(GlorpExec::Txn(GlorpTxn { execs }))?;
				if preset != glorp_api::SamplePreset::Custom {
					self.execute_gui(GuiCommand::ViewportScrollTo { x: 0.0, y: 0.0 })?;
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
			ControlsMessage::ShowBaselinesChanged(show_baselines) => {
				self.execute_gui(GuiCommand::ShowBaselinesSet(show_baselines))
			}
			ControlsMessage::ShowHitboxesChanged(show_hitboxes) => {
				self.execute_gui(GuiCommand::ShowHitboxesSet(show_hitboxes))
			}
		}
	}

	fn handle_canvas(&mut self, message: CanvasEvent) -> Result<(), GlorpError> {
		match message {
			CanvasEvent::Hovered(target) => self.execute_gui(GuiCommand::InspectTargetHover(inspect_target(
				self.frame.ui.active_tab,
				target,
			))),
			CanvasEvent::FocusChanged(focused) => self.execute_gui(GuiCommand::CanvasFocusSet(focused)),
			CanvasEvent::ScrollChanged(scroll) => self.execute_gui(GuiCommand::ViewportScrollTo {
				x: scroll.x,
				y: scroll.y,
			}),
			CanvasEvent::PointerSelectionStarted { target, intent } => {
				let active_tab = self.frame.ui.active_tab;
				self.mutate_host(|host| {
					host.execute_gui(GuiCommand::CanvasFocusSet(true))?;
					host.execute_gui(GuiCommand::InspectTargetSelect(inspect_target(active_tab, target)))?;
					host.execute_gui(pointer_command(intent))
				})
			}
		}
	}

	fn handle_editor(&mut self, intent: EditorIntent) -> Result<(), GlorpError> {
		match intent {
			EditorIntent::Pointer(pointer) => self.execute_gui(pointer_command(pointer)),
			intent => self.execute(editor_intent_command(intent)),
		}
	}

	fn execute(&mut self, exec: GlorpExec) -> Result<(), GlorpError> {
		self.mutate_host(|host| {
			host.execute(exec)?;
			Ok(())
		})
	}

	fn execute_gui(&mut self, command: GuiCommand) -> Result<(), GlorpError> {
		self.mutate_host(|host| host.execute_gui(command))
	}

	fn refresh_frame(&mut self) -> Result<(), GlorpError> {
		self.perf.flush_canvas_metrics();
		let mut frame = self.with_host(|host| {
			let mut frame = host.gui_frame();
			if scene_required(&frame) && frame.snapshot.scene.is_none() {
				host.execute_gui(GuiCommand::SceneEnsure)?;
				frame = host.gui_frame();
			}
			Ok(frame)
		})?;

		if let Some(scroll) = reveal_scroll(&frame) {
			frame = self.with_host(|host| {
				host.execute_gui(GuiCommand::ViewportScrollTo {
					x: scroll.x,
					y: scroll.y,
				})?;
				Ok(host.gui_frame())
			})?;
		}

		self.frame = frame;
		Ok(())
	}

	fn mutate_host(&mut self, f: impl FnOnce(&mut RuntimeHost) -> Result<(), GlorpError>) -> Result<(), GlorpError> {
		self.with_host(f)?;
		self.refresh_frame()
	}

	fn with_host<T>(&self, f: impl FnOnce(&mut RuntimeHost) -> Result<T, GlorpError>) -> Result<T, GlorpError> {
		let mut host = self
			.host
			.lock()
			.map_err(|_| GlorpError::transport("GUI runtime lock poisoned"))?;
		f(&mut host)
	}
}

fn config_set(path: &str, value: GlorpValue) -> GlorpExec {
	GlorpExec::ConfigSet(ConfigAssignment {
		path: path.to_owned(),
		value,
	})
}

fn enum_value<T>(value: T) -> GlorpValue
where
	T: EnumValue, {
	value.as_ref().into()
}

fn inspect_target(active_tab: SidebarTab, target: Option<CanvasTarget>) -> Option<CanvasTarget> {
	target.filter(|_| active_tab == SidebarTab::Inspect)
}

const fn scene_required(frame: &GuiRuntimeFrame) -> bool {
	matches!(frame.ui.active_tab, SidebarTab::Inspect | SidebarTab::Perf)
		|| frame.ui.show_baselines
		|| frame.ui.show_hitboxes
}

fn reveal_scroll(frame: &GuiRuntimeFrame) -> Option<Vector> {
	let target = frame.snapshot.editor.editor.viewport_target?;
	let metrics = frame.snapshot.editor.viewport_metrics;
	let viewport = Size::new(frame.ui.viewport_width.max(1.0), frame.ui.viewport_height.max(1.0));
	let current = Vector::new(frame.ui.canvas_scroll_x, frame.ui.canvas_scroll_y);
	let next = reveal_target_scroll(current, target, metrics, frame.ui.layout_width, viewport);
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

fn editor_intent_command(intent: EditorIntent) -> GlorpExec {
	match intent {
		EditorIntent::Pointer(_) => unreachable!("pointer intents stay on the private GUI command path"),
		EditorIntent::Motion(motion) => GlorpExec::EditorMotion(EditorMotionInput {
			motion: match motion {
				EditorMotion::Left => glorp_api::EditorMotion::Left,
				EditorMotion::Right => glorp_api::EditorMotion::Right,
				EditorMotion::Up => glorp_api::EditorMotion::Up,
				EditorMotion::Down => glorp_api::EditorMotion::Down,
				EditorMotion::LineStart => glorp_api::EditorMotion::LineStart,
				EditorMotion::LineEnd => glorp_api::EditorMotion::LineEnd,
			},
		}),
		EditorIntent::Mode(mode) => GlorpExec::EditorMode(EditorModeInput {
			mode: match mode {
				EditorModeIntent::EnterInsertBefore => glorp_api::EditorModeCommand::EnterInsertBefore,
				EditorModeIntent::EnterInsertAfter => glorp_api::EditorModeCommand::EnterInsertAfter,
				EditorModeIntent::ExitInsert => glorp_api::EditorModeCommand::ExitInsert,
			},
		}),
		EditorIntent::Edit(edit) => match edit {
			EditorEditIntent::Backspace => GlorpExec::EditorBackspace,
			EditorEditIntent::DeleteForward => GlorpExec::EditorDeleteForward,
			EditorEditIntent::DeleteSelection => GlorpExec::EditorDeleteSelection,
			EditorEditIntent::InsertText(text) => GlorpExec::EditorInsert(TextInput { text }),
		},
		EditorIntent::History(history) => GlorpExec::EditorHistory(EditorHistoryInput {
			action: match history {
				EditorHistoryIntent::Undo => glorp_api::EditorHistoryCommand::Undo,
				EditorHistoryIntent::Redo => glorp_api::EditorHistoryCommand::Redo,
			},
		}),
	}
}

fn pointer_command(pointer: EditorPointerIntent) -> GuiCommand {
	match pointer {
		EditorPointerIntent::Begin { position, select_word } => GuiCommand::EditorPointerBegin {
			x: position.x,
			y: position.y,
			select_word,
		},
		EditorPointerIntent::Drag(position) => GuiCommand::EditorPointerDrag {
			x: position.x,
			y: position.y,
		},
		EditorPointerIntent::End => GuiCommand::EditorPointerEnd,
	}
}
