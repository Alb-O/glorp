use {
	clap::Parser,
	glorp_api::{
		DocumentCommand, EditorCommand, EditorEditCommand, EditorHistoryCommand, EditorMode, EditorModeCommand,
		EditorMotion, GlorpCommand, GlorpError, GlorpHost, GlorpOutcome, GlorpQuery, GlorpQueryResult, GlorpSnapshot,
		SceneCommand, SidebarTab, UiCommand,
	},
	glorp_gui::{GuiLaunchOptions, GuiPresentation, GuiRuntimeSession},
	glorp_transport::IpcClient,
	iced::{
		Center, Element, Fill, Font, Subscription, Task, Theme, event,
		keyboard::{self, key},
		time::{self, Duration},
		widget::{button, column, container, row, scrollable, text},
	},
	std::{path::PathBuf, process::ExitCode, sync::Mutex},
};

fn main() -> ExitCode {
	match run() {
		Ok(()) => ExitCode::SUCCESS,
		Err(error) => {
			eprintln!("{error}");
			ExitCode::FAILURE
		}
	}
}

fn run() -> Result<(), GlorpError> {
	let cli = Cli::parse();
	let repo_root = cli.repo_root.unwrap_or(
		std::env::current_dir()
			.map_err(|error| GlorpError::transport(format!("failed to determine current directory: {error}")))?,
	);
	let mut launch = GuiLaunchOptions::for_repo_root(repo_root.clone());
	if let Some(socket_path) = cli.socket {
		launch.socket_path = socket_path;
	}

	let (session, client) = GuiRuntimeSession::connect_or_start(launch)?;
	eprintln!("glorp_gui listening on {}", session.socket_path().display());
	let boot = Mutex::new(Some((session, client, repo_root, cli.poll_ms)));

	iced::application(
		move || {
			let (session, client, repo_root, poll_ms) = boot
				.lock()
				.expect("GUI boot mutex should not be poisoned")
				.take()
				.expect("GUI boot state should only initialize once");
			RuntimeEditor::new(session, client, repo_root, poll_ms)
		},
		RuntimeEditor::update,
		RuntimeEditor::view,
	)
	.subscription(RuntimeEditor::subscription)
	.theme(RuntimeEditor::theme)
	.window_size([1200.0, 820.0])
	.centered()
	.run()
	.map_err(|error| GlorpError::internal(format!("GUI application failed: {error}")))
}

#[derive(Debug, Parser, Clone)]
struct Cli {
	#[arg(long)]
	socket: Option<PathBuf>,
	#[arg(long)]
	repo_root: Option<PathBuf>,
	#[arg(long, default_value_t = 250)]
	poll_ms: u64,
}

struct RuntimeEditor {
	session: GuiRuntimeSession,
	client: IpcClient,
	repo_root: PathBuf,
	socket_path: PathBuf,
	poll_ms: u64,
	presentation: Option<GuiPresentation>,
	document_text: String,
	status: String,
	last_error: Option<String>,
	refreshing: bool,
}

#[derive(Debug, Clone)]
enum Message {
	Tick,
	EventOccurred(iced::Event),
	Refresh,
	Refreshed(Result<RefreshState, String>),
	Execute(GlorpCommand),
	CommandFinished(Result<GlorpOutcome, String>),
}

#[derive(Debug, Clone)]
struct RefreshState {
	snapshot: GlorpSnapshot,
	document_text: String,
}

impl RuntimeEditor {
	fn new(session: GuiRuntimeSession, client: IpcClient, repo_root: PathBuf, poll_ms: u64) -> (Self, Task<Message>) {
		let socket_path = session.socket_path().to_path_buf();
		let status = if session.owns_server() {
			format!("hosting shared runtime at {}", socket_path.display())
		} else {
			format!("joined shared runtime at {}", socket_path.display())
		};
		let app = Self {
			session,
			client: client.clone(),
			repo_root,
			socket_path,
			poll_ms,
			presentation: None,
			document_text: String::new(),
			status,
			last_error: None,
			refreshing: true,
		};
		(app, refresh_task(client))
	}

	fn update(&mut self, message: Message) -> Task<Message> {
		match message {
			Message::Tick | Message::Refresh => {
				if self.refreshing {
					return Task::none();
				}
				self.refreshing = true;
				refresh_task(self.client.clone())
			}
			Message::Refreshed(result) => {
				self.refreshing = false;
				match result {
					Ok(state) => {
						self.presentation = Some(GuiPresentation {
							snapshot: state.snapshot,
						});
						self.document_text = state.document_text;
						self.last_error = None;
					}
					Err(error) => self.last_error = Some(error),
				}
				Task::none()
			}
			Message::Execute(command) => {
				self.status = describe_command(&command);
				Task::perform(execute_command(self.client.clone(), command), Message::CommandFinished)
			}
			Message::CommandFinished(result) => {
				match result {
					Ok(outcome) => {
						self.status = format!(
							"applied command at revisions c={} e={} s={:?}",
							outcome.revisions.config, outcome.revisions.editor, outcome.revisions.scene
						);
						self.last_error = None;
					}
					Err(error) => self.last_error = Some(error),
				}
				if self.refreshing {
					Task::none()
				} else {
					self.refreshing = true;
					refresh_task(self.client.clone())
				}
			}
			Message::EventOccurred(event) => self
				.command_from_event(&event)
				.map_or_else(Task::none, |command| self.update(Message::Execute(command))),
		}
	}

	fn subscription(&self) -> Subscription<Message> {
		Subscription::batch([
			time::every(Duration::from_millis(self.poll_ms)).map(|_| Message::Tick),
			event::listen().map(Message::EventOccurred),
		])
	}

	fn theme(&self) -> Theme {
		Theme::TokyoNightStorm
	}

	fn view(&self) -> Element<'_, Message> {
		let mode = self
			.presentation
			.as_ref()
			.map(|presentation| presentation.snapshot.editor.mode)
			.unwrap_or(EditorMode::Normal);
		let active_tab = self
			.presentation
			.as_ref()
			.map(|presentation| presentation.snapshot.ui.active_tab)
			.unwrap_or(SidebarTab::Controls);
		let summary = self.presentation.as_ref().map_or_else(
			|| "waiting for runtime snapshot".to_owned(),
			|presentation| {
				let snapshot = &presentation.snapshot;
				format!(
					"mode={} bytes={} lines={} undo={} redo={} tab={} wrapping={} pane={:.2}",
					mode_label(snapshot.editor.mode),
					snapshot.editor.text_bytes,
					snapshot.editor.text_lines,
					snapshot.editor.undo_depth,
					snapshot.editor.redo_depth,
					tab_label(snapshot.ui.active_tab),
					wrap_label(snapshot.config.editor.wrapping),
					snapshot.ui.pane_ratio,
				)
			},
		);
		let revisions = self.presentation.as_ref().map_or_else(
			|| "revisions unavailable".to_owned(),
			|presentation| {
				let revisions = presentation.snapshot.revisions;
				format!(
					"revisions config={} editor={} scene={:?}",
					revisions.config, revisions.editor, revisions.scene
				)
			},
		);
		let scene = self.presentation.as_ref().map_or_else(
			|| "scene unavailable".to_owned(),
			|presentation| match &presentation.snapshot.scene {
				Some(scene) => format!(
					"scene rev={} runs={} clusters={} size=({:.0}x{:.0})",
					scene.revision, scene.run_count, scene.cluster_count, scene.measured_width, scene.measured_height
				),
				None => "scene omitted".to_owned(),
			},
		);
		let inspect = self.presentation.as_ref().map_or_else(
			|| "inspect unavailable".to_owned(),
			|presentation| {
				format!(
					"inspect hovered={:?} selected={:?} scroll=({:.0}, {:.0})",
					presentation.snapshot.inspect.hovered_target,
					presentation.snapshot.inspect.selected_target,
					presentation.snapshot.ui.canvas_scroll_x,
					presentation.snapshot.ui.canvas_scroll_y,
				)
			},
		);

		let tabs = row![
			tab_button("Controls", SidebarTab::Controls, active_tab),
			tab_button("Inspect", SidebarTab::Inspect, active_tab),
			tab_button("Perf", SidebarTab::Perf, active_tab),
			button("Refresh").on_press(Message::Refresh),
			button("Scene").on_press(Message::Execute(GlorpCommand::Scene(SceneCommand::Ensure))),
		]
		.spacing(8)
		.align_y(Center);

		let modes = row![
			button("Insert Before").on_press(Message::Execute(GlorpCommand::Editor(EditorCommand::Mode(
				EditorModeCommand::EnterInsertBefore,
			)))),
			button("Insert After").on_press(Message::Execute(GlorpCommand::Editor(EditorCommand::Mode(
				EditorModeCommand::EnterInsertAfter,
			)))),
			button("Normal").on_press(Message::Execute(GlorpCommand::Editor(EditorCommand::Mode(
				EditorModeCommand::ExitInsert,
			)))),
			button("Undo").on_press(Message::Execute(GlorpCommand::Editor(EditorCommand::History(
				EditorHistoryCommand::Undo,
			)))),
			button("Redo").on_press(Message::Execute(GlorpCommand::Editor(EditorCommand::History(
				EditorHistoryCommand::Redo,
			)))),
		]
		.spacing(8)
		.align_y(Center);

		let motion = row![
			button("Left").on_press(Message::Execute(editor_motion(EditorMotion::Left))),
			button("Right").on_press(Message::Execute(editor_motion(EditorMotion::Right))),
			button("Up").on_press(Message::Execute(editor_motion(EditorMotion::Up))),
			button("Down").on_press(Message::Execute(editor_motion(EditorMotion::Down))),
			button("Home").on_press(Message::Execute(editor_motion(EditorMotion::LineStart))),
			button("End").on_press(Message::Execute(editor_motion(EditorMotion::LineEnd))),
			button("Backspace").on_press(Message::Execute(editor_edit(EditorEditCommand::Backspace))),
			button("Delete").on_press(Message::Execute(editor_edit(EditorEditCommand::DeleteForward))),
			button("Newline").on_press(Message::Execute(editor_edit(EditorEditCommand::Insert {
				text: "\n".to_owned(),
			}))),
		]
		.spacing(8)
		.align_y(Center);

		let metadata = column![
			text(format!("repo root: {}", self.repo_root.display())).size(16),
			text(format!("socket: {}", self.socket_path.display())).size(16),
			text(if self.session.owns_server() {
				"runtime ownership: hosting".to_owned()
			} else {
				"runtime ownership: attached".to_owned()
			})
			.size(16),
			text(format!("status: {}", self.status)).size(16),
			text(summary).size(16),
			text(revisions).size(16),
			text(scene).size(16),
			text(inspect).size(16),
			text(format!(
				"keys: i/a enter insert, esc normal, arrows move, home/end line, backspace/delete edit, ctrl+z undo, ctrl+y redo"
			))
			.size(16),
			text(format!("window mode: {}", mode_label(mode))).size(16),
		]
		.spacing(6);

		let document = scrollable(
			container(text(&self.document_text).font(Font::MONOSPACE).size(18).width(Fill))
				.padding(16)
				.width(Fill),
		)
		.height(Fill);

		let mut content = column![tabs, modes, motion, metadata, document].spacing(14).padding(18);
		if let Some(error) = &self.last_error {
			content = content.push(text(format!("error: {error}")).size(16));
		}

		container(content).width(Fill).height(Fill).into()
	}

	fn command_from_event(&self, event: &iced::Event) -> Option<GlorpCommand> {
		let mode = self.presentation.as_ref()?.snapshot.editor.mode;
		let iced::Event::Keyboard(keyboard::Event::KeyPressed {
			key, modifiers, text, ..
		}) = event
		else {
			return None;
		};

		if modifiers.command() || modifiers.control() {
			return match key.as_ref() {
				keyboard::Key::Character("z") if modifiers.shift() => {
					Some(GlorpCommand::Editor(EditorCommand::History(EditorHistoryCommand::Redo)))
				}
				keyboard::Key::Character("z") => {
					Some(GlorpCommand::Editor(EditorCommand::History(EditorHistoryCommand::Undo)))
				}
				keyboard::Key::Character("y") => {
					Some(GlorpCommand::Editor(EditorCommand::History(EditorHistoryCommand::Redo)))
				}
				_ => None,
			};
		}

		match key.as_ref() {
			keyboard::Key::Named(key::Named::Escape) => {
				Some(GlorpCommand::Editor(EditorCommand::Mode(EditorModeCommand::ExitInsert)))
			}
			keyboard::Key::Named(key::Named::ArrowLeft) => Some(editor_motion(EditorMotion::Left)),
			keyboard::Key::Named(key::Named::ArrowRight) => Some(editor_motion(EditorMotion::Right)),
			keyboard::Key::Named(key::Named::ArrowUp) => Some(editor_motion(EditorMotion::Up)),
			keyboard::Key::Named(key::Named::ArrowDown) => Some(editor_motion(EditorMotion::Down)),
			keyboard::Key::Named(key::Named::Home) => Some(editor_motion(EditorMotion::LineStart)),
			keyboard::Key::Named(key::Named::End) => Some(editor_motion(EditorMotion::LineEnd)),
			keyboard::Key::Named(key::Named::Backspace) => Some(editor_edit(EditorEditCommand::Backspace)),
			keyboard::Key::Named(key::Named::Delete) => Some(editor_edit(EditorEditCommand::DeleteForward)),
			keyboard::Key::Named(key::Named::Enter) if mode == EditorMode::Insert => {
				Some(editor_edit(EditorEditCommand::Insert { text: "\n".to_owned() }))
			}
			keyboard::Key::Character("i") if mode == EditorMode::Normal => Some(GlorpCommand::Editor(
				EditorCommand::Mode(EditorModeCommand::EnterInsertBefore),
			)),
			keyboard::Key::Character("a") if mode == EditorMode::Normal => Some(GlorpCommand::Editor(
				EditorCommand::Mode(EditorModeCommand::EnterInsertAfter),
			)),
			keyboard::Key::Character("u") if mode == EditorMode::Normal => {
				Some(GlorpCommand::Editor(EditorCommand::History(EditorHistoryCommand::Undo)))
			}
			_ if mode == EditorMode::Insert && !modifiers.alt() => text.as_ref().and_then(|text| {
				(!text.is_empty()).then(|| editor_edit(EditorEditCommand::Insert { text: text.to_string() }))
			}),
			_ => None,
		}
	}
}

async fn refresh_state(mut client: IpcClient) -> Result<RefreshState, String> {
	let snapshot = match client
		.query(GlorpQuery::Snapshot {
			scene: glorp_api::SceneLevel::Materialize,
			include_document_text: false,
		})
		.map_err(|error| error.to_string())?
	{
		GlorpQueryResult::Snapshot(snapshot) => snapshot,
		other => return Err(format!("unexpected snapshot response: {other:?}")),
	};
	let document_text = match client
		.query(GlorpQuery::DocumentText)
		.map_err(|error| error.to_string())?
	{
		GlorpQueryResult::DocumentText(text) => text,
		other => return Err(format!("unexpected document response: {other:?}")),
	};
	Ok(RefreshState {
		snapshot,
		document_text,
	})
}

async fn execute_command(mut client: IpcClient, command: GlorpCommand) -> Result<GlorpOutcome, String> {
	client.execute(command).map_err(|error| error.to_string())
}

fn refresh_task(client: IpcClient) -> Task<Message> {
	Task::perform(refresh_state(client), Message::Refreshed)
}

fn tab_button(label: &'static str, tab: SidebarTab, active_tab: SidebarTab) -> Element<'static, Message> {
	let button = button(text(label));
	if tab == active_tab {
		button.into()
	} else {
		button
			.on_press(Message::Execute(GlorpCommand::Ui(UiCommand::SidebarSelect { tab })))
			.into()
	}
}

fn editor_motion(motion: EditorMotion) -> GlorpCommand {
	GlorpCommand::Editor(EditorCommand::Motion(motion))
}

fn editor_edit(command: EditorEditCommand) -> GlorpCommand {
	GlorpCommand::Editor(EditorCommand::Edit(command))
}

fn describe_command(command: &GlorpCommand) -> String {
	match command {
		GlorpCommand::Config(_) => "updating config".to_owned(),
		GlorpCommand::Document(DocumentCommand::Replace { .. }) => "replacing document text".to_owned(),
		GlorpCommand::Editor(EditorCommand::Mode(mode)) => format!("changing mode to {mode:?}"),
		GlorpCommand::Editor(EditorCommand::Motion(motion)) => format!("moving cursor with {motion:?}"),
		GlorpCommand::Editor(EditorCommand::Edit(_)) => "editing document".to_owned(),
		GlorpCommand::Editor(EditorCommand::History(history)) => format!("history action {history:?}"),
		GlorpCommand::Editor(EditorCommand::Pointer(_)) => "pointer command".to_owned(),
		GlorpCommand::Ui(UiCommand::SidebarSelect { tab }) => format!("selecting tab {}", tab_label(*tab)),
		GlorpCommand::Ui(UiCommand::InspectTargetSelect { .. }) => "selecting inspect target".to_owned(),
		GlorpCommand::Ui(UiCommand::ViewportScrollTo { .. }) => "scrolling viewport".to_owned(),
		GlorpCommand::Ui(UiCommand::PaneRatioSet { ratio }) => format!("setting pane ratio to {ratio:.2}"),
		GlorpCommand::Scene(SceneCommand::Ensure) => "materializing scene".to_owned(),
		GlorpCommand::Txn(_) => "executing transaction".to_owned(),
	}
}

fn mode_label(mode: EditorMode) -> &'static str {
	match mode {
		EditorMode::Normal => "normal",
		EditorMode::Insert => "insert",
	}
}

fn tab_label(tab: SidebarTab) -> &'static str {
	match tab {
		SidebarTab::Controls => "controls",
		SidebarTab::Inspect => "inspect",
		SidebarTab::Perf => "perf",
	}
}

fn wrap_label(wrap: glorp_api::WrapChoice) -> &'static str {
	match wrap {
		glorp_api::WrapChoice::None => "none",
		glorp_api::WrapChoice::Word => "word",
		glorp_api::WrapChoice::Glyph => "glyph",
		glorp_api::WrapChoice::WordOrGlyph => "word-or-glyph",
	}
}
