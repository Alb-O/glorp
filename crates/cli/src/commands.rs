use {
	crate::output,
	clap::{Parser, Subcommand},
	glorp_api::{
		CanvasTarget, ConfigAssignment, ConfigCommand, DocumentCommand, EditorCommand, EditorEditCommand,
		EditorHistoryCommand, EditorModeCommand, EditorMotion, GlorpCapabilities, GlorpCommand, GlorpConfig,
		GlorpError, GlorpEvent, GlorpEventStreamView, GlorpHost, GlorpOutcome, GlorpQuery, GlorpQueryResult,
		GlorpSessionView, GlorpStreamToken, GlorpSubscription, GlorpTxn, GlorpValue, SceneCommand, SceneLevel,
		SidebarTab, UiCommand,
	},
	glorp_runtime::{RuntimeHost, RuntimeOptions, default_runtime_paths},
	glorp_transport::{IpcClient, default_socket_path, socket_is_live},
	std::path::{Path, PathBuf},
};

#[derive(Debug, Parser)]
pub struct Cli {
	#[arg(long)]
	socket: Option<PathBuf>,
	#[arg(long)]
	repo_root: Option<PathBuf>,
	#[command(subcommand)]
	command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
	Schema,
	Get {
		#[command(subcommand)]
		target: GetTarget,
	},
	Session {
		#[command(subcommand)]
		command: SessionSubcommand,
	},
	Config {
		#[command(subcommand)]
		command: ConfigSubcommand,
	},
	Doc {
		#[command(subcommand)]
		command: DocumentSubcommand,
	},
	Editor {
		#[command(subcommand)]
		command: EditorSubcommand,
	},
	Ui {
		#[command(subcommand)]
		command: UiSubcommand,
	},
	Scene {
		#[command(subcommand)]
		command: SceneSubcommand,
	},
	Events {
		#[command(subcommand)]
		command: EventsSubcommand,
	},
	Txn {
		json: String,
	},
}

#[derive(Debug, Subcommand)]
enum GetTarget {
	Config,
	State,
	DocumentText,
	Selection,
	InspectDetails {
		#[arg(long)]
		target: Option<String>,
	},
	Perf,
	Ui,
	Capabilities,
}

#[derive(Debug, Subcommand)]
enum SessionSubcommand {
	Attach,
}

#[derive(Debug, Subcommand)]
enum ConfigSubcommand {
	Set { path: String, value: String },
	Reset { path: String },
	Patch { json: String },
	Validate { path: String, value: String },
	Reload,
	Persist,
}

#[derive(Debug, Subcommand)]
enum DocumentSubcommand {
	Replace { text: String },
}

#[derive(Debug, Subcommand)]
enum EditorSubcommand {
	Motion {
		motion: String,
	},
	Mode {
		mode: String,
	},
	Edit {
		#[command(subcommand)]
		command: EditorEditSubcommand,
	},
	History {
		action: String,
	},
}

#[derive(Debug, Subcommand)]
enum EditorEditSubcommand {
	Insert { text: String },
	Backspace,
	DeleteForward,
	DeleteSelection,
}

#[derive(Debug, Subcommand)]
enum UiSubcommand {
	Sidebar {
		#[command(subcommand)]
		command: UiSidebarSubcommand,
	},
	Viewport {
		#[command(subcommand)]
		command: UiViewportSubcommand,
	},
	PaneRatioSet {
		ratio: f32,
	},
}

#[derive(Debug, Subcommand)]
enum UiSidebarSubcommand {
	Select { tab: String },
}

#[derive(Debug, Subcommand)]
enum UiViewportSubcommand {
	ScrollTo { x: f32, y: f32 },
}

#[derive(Debug, Subcommand)]
enum SceneSubcommand {
	Ensure,
}

#[derive(Debug, Clone, Copy, Subcommand)]
enum EventsSubcommand {
	Subscribe,
	Next { token: u64 },
	Unsubscribe { token: u64 },
}

enum Host {
	Local(Box<RuntimeHost>),
	Ipc(IpcClient),
}

impl Cli {
	pub fn run(self) -> Result<(), GlorpError> {
		let mut host = self.host()?;
		self.run_with_host(&mut host)
	}

	fn run_with_host(self, host: &mut Host) -> Result<(), GlorpError> {
		match self.command {
			Command::Schema => output::print_query(&host.query(GlorpQuery::Schema)?)?,
			Command::Get { target } => output::print_query(&host.query(query_for_target(target)?)?)?,
			Command::Session {
				command: SessionSubcommand::Attach,
			} => output::print_json(&self.attach_session()?)?,
			Command::Config { command } => run_config(host, command)?,
			Command::Doc {
				command: DocumentSubcommand::Replace { text },
			} => output::print_outcome(&host.execute(GlorpCommand::Document(DocumentCommand::Replace { text }))?)?,
			Command::Editor { command } => run_editor(host, command)?,
			Command::Ui { command } => run_ui(host, command)?,
			Command::Scene {
				command: SceneSubcommand::Ensure,
			} => output::print_outcome(&host.execute(GlorpCommand::Scene(SceneCommand::Ensure))?)?,
			Command::Events { command } => run_events(host, command)?,
			Command::Txn { json } => run_txn(host, &json)?,
		}

		Ok(())
	}

	fn host(&self) -> Result<Host, GlorpError> {
		if let Some(socket) = self.requested_socket() {
			return Ok(Host::Ipc(IpcClient::new(socket)));
		}

		let repo_root = self.repo_root_or_cwd()?;
		if let Some(socket) = autodetect_socket(&repo_root) {
			return Ok(Host::Ipc(IpcClient::new(socket)));
		}
		let options = RuntimeOptions {
			paths: default_runtime_paths(repo_root),
		};
		Ok(Host::Local(Box::new(RuntimeHost::new(options)?)))
	}

	fn attach_session(&self) -> Result<GlorpSessionView, GlorpError> {
		let (socket, repo_root) = self.live_socket()?;
		let mut client = IpcClient::new(socket.clone());
		let capabilities = query_capabilities(&mut client, &socket)?;
		Ok(GlorpSessionView {
			socket: socket.display().to_string(),
			repo_root: repo_root.map(|repo_root| repo_root.display().to_string()),
			capabilities,
		})
	}

	fn live_socket(&self) -> Result<(PathBuf, Option<PathBuf>), GlorpError> {
		if let Some(socket) = self.requested_socket() {
			let display = socket.display().to_string();
			return socket_is_live(&socket)
				.then_some((socket, None))
				.ok_or_else(|| GlorpError::transport(format!("no live runtime at {display}")));
		}

		let repo_root = self.repo_root_or_cwd()?;
		let socket = default_socket_path(&repo_root);
		let display = socket.display().to_string();
		socket_is_live(&socket)
			.then_some((socket, Some(repo_root)))
			.ok_or_else(|| GlorpError::transport(format!("no live runtime at {display}")))
	}

	fn requested_socket(&self) -> Option<PathBuf> {
		self.socket
			.clone()
			.or_else(|| std::env::var_os("GLORP_SOCKET").map(PathBuf::from))
	}

	fn repo_root_or_cwd(&self) -> Result<PathBuf, GlorpError> {
		self.repo_root.clone().map_or_else(
			|| {
				std::env::current_dir()
					.map_err(|error| GlorpError::transport(format!("failed to determine current directory: {error}")))
			},
			Ok,
		)
	}
}

fn autodetect_socket(repo_root: &Path) -> Option<PathBuf> {
	let socket = default_socket_path(repo_root);
	socket_is_live(&socket).then_some(socket)
}

fn query_for_target(target: GetTarget) -> Result<GlorpQuery, GlorpError> {
	Ok(match target {
		GetTarget::Config => GlorpQuery::Config,
		GetTarget::State => GlorpQuery::Snapshot {
			scene: SceneLevel::Materialize,
			include_document_text: true,
		},
		GetTarget::DocumentText => GlorpQuery::DocumentText,
		GetTarget::Selection => GlorpQuery::Selection,
		GetTarget::InspectDetails { target } => GlorpQuery::InspectDetails {
			target: target.as_deref().map(parse_canvas_target).transpose()?,
		},
		GetTarget::Perf => GlorpQuery::PerfDashboard,
		GetTarget::Ui => GlorpQuery::UiState,
		GetTarget::Capabilities => GlorpQuery::Capabilities,
	})
}

fn run_config(host: &mut Host, command: ConfigSubcommand) -> Result<(), GlorpError> {
	match command {
		ConfigSubcommand::Set { path, value } => {
			let value = parse_value(&value);
			output::print_outcome(&host.execute(GlorpCommand::Config(ConfigCommand::Set { path, value }))?)?;
		}
		ConfigSubcommand::Reset { path } => {
			output::print_outcome(&host.execute(GlorpCommand::Config(ConfigCommand::Reset { path }))?)?;
		}
		ConfigSubcommand::Patch { json } => {
			let value: serde_json::Value = serde_json::from_str(&json)
				.map_err(|error| GlorpError::validation(None, format!("invalid patch JSON: {error}")))?;
			let assignments = flatten_patch(&GlorpValue::from(value))?;
			output::print_outcome(&host.execute(GlorpCommand::Config(ConfigCommand::Patch { values: assignments }))?)?;
		}
		ConfigSubcommand::Validate { path, value } => {
			GlorpConfig::validate_path(&path, parse_value(&value))?;
			output::print_json(&serde_json::json!({ "ok": true }))?;
		}
		ConfigSubcommand::Reload => {
			output::print_outcome(&host.execute(GlorpCommand::Config(ConfigCommand::Reload))?)?;
		}
		ConfigSubcommand::Persist => {
			output::print_outcome(&host.execute(GlorpCommand::Config(ConfigCommand::Persist))?)?;
		}
	}

	Ok(())
}

fn run_editor(host: &mut Host, command: EditorSubcommand) -> Result<(), GlorpError> {
	let command = match command {
		EditorSubcommand::Motion { motion } => GlorpCommand::Editor(EditorCommand::Motion(parse_motion(&motion)?)),
		EditorSubcommand::Mode { mode } => GlorpCommand::Editor(EditorCommand::Mode(parse_mode(&mode)?)),
		EditorSubcommand::Edit { command } => GlorpCommand::Editor(EditorCommand::Edit(match command {
			EditorEditSubcommand::Insert { text } => EditorEditCommand::Insert { text },
			EditorEditSubcommand::Backspace => EditorEditCommand::Backspace,
			EditorEditSubcommand::DeleteForward => EditorEditCommand::DeleteForward,
			EditorEditSubcommand::DeleteSelection => EditorEditCommand::DeleteSelection,
		})),
		EditorSubcommand::History { action } => GlorpCommand::Editor(EditorCommand::History(parse_history(&action)?)),
	};
	output::print_outcome(&host.execute(command)?)?;
	Ok(())
}

fn run_ui(host: &mut Host, command: UiSubcommand) -> Result<(), GlorpError> {
	let command = match command {
		UiSubcommand::Sidebar {
			command: UiSidebarSubcommand::Select { tab },
		} => GlorpCommand::Ui(UiCommand::SidebarSelect { tab: parse_tab(&tab)? }),
		UiSubcommand::Viewport {
			command: UiViewportSubcommand::ScrollTo { x, y },
		} => GlorpCommand::Ui(UiCommand::ViewportScrollTo { x, y }),
		UiSubcommand::PaneRatioSet { ratio } => GlorpCommand::Ui(UiCommand::PaneRatioSet { ratio }),
	};
	output::print_outcome(&host.execute(command)?)?;
	Ok(())
}

fn run_events(host: &mut Host, command: EventsSubcommand) -> Result<(), GlorpError> {
	match command {
		EventsSubcommand::Subscribe => {
			let token = host.subscribe(GlorpSubscription::Changes)?;
			output::print_json(&GlorpEventStreamView {
				token,
				subscription: "changes".into(),
			})?;
		}
		EventsSubcommand::Next { token } => output::print_json(&host.next_event(token)?)?,
		EventsSubcommand::Unsubscribe { token } => {
			host.unsubscribe(token)?;
			output::print_json(&serde_json::json!({
				"ok": true,
				"token": token,
			}))?;
		}
	}

	Ok(())
}

fn run_txn(host: &mut Host, json: &str) -> Result<(), GlorpError> {
	let txn: GlorpTxn = serde_json::from_str(json)
		.map_err(|error| GlorpError::validation(None, format!("invalid txn JSON: {error}")))?;
	output::print_outcome(&host.execute(GlorpCommand::Txn(txn))?)?;
	Ok(())
}

fn query_capabilities(host: &mut impl GlorpHost, socket: &Path) -> Result<GlorpCapabilities, GlorpError> {
	let GlorpQueryResult::Capabilities(capabilities) = host.query(GlorpQuery::Capabilities)? else {
		return Err(GlorpError::transport(format!(
			"unexpected capabilities response from {}",
			socket.display()
		)));
	};

	Ok(capabilities)
}

impl Host {
	fn as_glorp_host(&mut self) -> &mut dyn GlorpHost {
		match self {
			Self::Local(host) => host.as_mut(),
			Self::Ipc(host) => host,
		}
	}
}

impl GlorpHost for Host {
	fn execute(&mut self, command: GlorpCommand) -> Result<GlorpOutcome, GlorpError> {
		self.as_glorp_host().execute(command)
	}

	fn query(&mut self, query: GlorpQuery) -> Result<GlorpQueryResult, GlorpError> {
		self.as_glorp_host().query(query)
	}

	fn subscribe(&mut self, request: GlorpSubscription) -> Result<GlorpStreamToken, GlorpError> {
		self.as_glorp_host().subscribe(request)
	}

	fn next_event(&mut self, token: GlorpStreamToken) -> Result<Option<GlorpEvent>, GlorpError> {
		self.as_glorp_host().next_event(token)
	}

	fn unsubscribe(&mut self, token: GlorpStreamToken) -> Result<(), GlorpError> {
		self.as_glorp_host().unsubscribe(token)
	}
}

fn parse_value(input: &str) -> GlorpValue {
	serde_json::from_str::<serde_json::Value>(input).map_or_else(|_| GlorpValue::String(input.into()), GlorpValue::from)
}

fn flatten_patch(value: &GlorpValue) -> Result<Vec<ConfigAssignment>, GlorpError> {
	let mut assignments = Vec::new();
	let mut path = String::new();
	flatten_patch_into(&mut assignments, &mut path, value)?;
	Ok(assignments)
}

fn flatten_patch_into(
	assignments: &mut Vec<ConfigAssignment>, path: &mut String, value: &GlorpValue,
) -> Result<(), GlorpError> {
	match value {
		GlorpValue::Record(fields) => fields.iter().try_for_each(|(key, value)| {
			let len = path.len();
			if !path.is_empty() {
				path.push('.');
			}
			path.push_str(key);
			let result = flatten_patch_into(assignments, path, value);
			path.truncate(len);
			result
		}),
		other => {
			assignments.push(ConfigAssignment {
				path: path.clone(),
				value: other.clone(),
			});
			Ok(())
		}
	}
}

fn parse_motion(value: &str) -> Result<EditorMotion, GlorpError> {
	match value {
		"left" => Ok(EditorMotion::Left),
		"right" => Ok(EditorMotion::Right),
		"up" => Ok(EditorMotion::Up),
		"down" => Ok(EditorMotion::Down),
		"line-start" => Ok(EditorMotion::LineStart),
		"line-end" => Ok(EditorMotion::LineEnd),
		_ => Err(GlorpError::validation(None, format!("unknown motion `{value}`"))),
	}
}

fn parse_mode(value: &str) -> Result<EditorModeCommand, GlorpError> {
	match value {
		"enter-insert-before" => Ok(EditorModeCommand::EnterInsertBefore),
		"enter-insert-after" => Ok(EditorModeCommand::EnterInsertAfter),
		"exit-insert" => Ok(EditorModeCommand::ExitInsert),
		_ => Err(GlorpError::validation(None, format!("unknown mode `{value}`"))),
	}
}

fn parse_history(value: &str) -> Result<EditorHistoryCommand, GlorpError> {
	match value {
		"undo" => Ok(EditorHistoryCommand::Undo),
		"redo" => Ok(EditorHistoryCommand::Redo),
		_ => Err(GlorpError::validation(
			None,
			format!("unknown history action `{value}`"),
		)),
	}
}

fn parse_tab(value: &str) -> Result<SidebarTab, GlorpError> {
	match value {
		"controls" => Ok(SidebarTab::Controls),
		"inspect" => Ok(SidebarTab::Inspect),
		"perf" => Ok(SidebarTab::Perf),
		_ => Err(GlorpError::validation(None, format!("unknown tab `{value}`"))),
	}
}

fn parse_canvas_target(value: &str) -> Result<CanvasTarget, GlorpError> {
	let (kind, index) = value
		.split_once(':')
		.ok_or_else(|| GlorpError::validation(None, format!("invalid canvas target `{value}`")))?;
	let index = index
		.parse::<usize>()
		.map_err(|error| GlorpError::validation(None, format!("invalid canvas target `{value}`: {error}")))?;
	match kind {
		"run" => Ok(CanvasTarget::Run(index)),
		"cluster" => Ok(CanvasTarget::Cluster(index)),
		_ => Err(GlorpError::validation(
			None,
			format!("unknown canvas target kind `{kind}`"),
		)),
	}
}
