use {
	crate::output,
	clap::{Parser, Subcommand},
	glorp_api::*,
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
	Txn {
		json: String,
	},
}

#[derive(Debug, Subcommand)]
enum GetTarget {
	Config,
	State,
	DocumentText,
	Capabilities,
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

enum Host {
	Local(RuntimeHost),
	Ipc(IpcClient),
}

impl Cli {
	pub fn run(self) -> Result<(), GlorpError> {
		let mut host = self.host()?;

		match self.command {
			Command::Schema => output::print_query(&host.query(GlorpQuery::Schema)?)?,
			Command::Get { target } => {
				let query = match target {
					GetTarget::Config => GlorpQuery::Config,
					GetTarget::State => GlorpQuery::Snapshot {
						scene: SceneLevel::Materialize,
						include_document_text: true,
					},
					GetTarget::DocumentText => GlorpQuery::DocumentText,
					GetTarget::Capabilities => GlorpQuery::Capabilities,
				};
				output::print_query(&host.query(query)?)?;
			}
			Command::Config { command } => match command {
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
					let assignments = flatten_patch(None, &GlorpValue::from(value))?;
					output::print_outcome(
						&host.execute(GlorpCommand::Config(ConfigCommand::Patch { values: assignments }))?,
					)?;
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
			},
			Command::Doc { command } => match command {
				DocumentSubcommand::Replace { text } => {
					output::print_outcome(&host.execute(GlorpCommand::Document(DocumentCommand::Replace { text }))?)?;
				}
			},
			Command::Editor { command } => {
				let command = match command {
					EditorSubcommand::Motion { motion } => {
						GlorpCommand::Editor(EditorCommand::Motion(parse_motion(&motion)?))
					}
					EditorSubcommand::Mode { mode } => GlorpCommand::Editor(EditorCommand::Mode(parse_mode(&mode)?)),
					EditorSubcommand::Edit { command } => GlorpCommand::Editor(EditorCommand::Edit(match command {
						EditorEditSubcommand::Insert { text } => EditorEditCommand::Insert { text },
						EditorEditSubcommand::Backspace => EditorEditCommand::Backspace,
						EditorEditSubcommand::DeleteForward => EditorEditCommand::DeleteForward,
						EditorEditSubcommand::DeleteSelection => EditorEditCommand::DeleteSelection,
					})),
					EditorSubcommand::History { action } => {
						GlorpCommand::Editor(EditorCommand::History(parse_history(&action)?))
					}
				};
				output::print_outcome(&host.execute(command)?)?;
			}
			Command::Ui { command } => {
				let command = match command {
					UiSubcommand::Sidebar { command } => match command {
						UiSidebarSubcommand::Select { tab } => {
							GlorpCommand::Ui(UiCommand::SidebarSelect { tab: parse_tab(&tab)? })
						}
					},
					UiSubcommand::Viewport { command } => match command {
						UiViewportSubcommand::ScrollTo { x, y } => {
							GlorpCommand::Ui(UiCommand::ViewportScrollTo { x, y })
						}
					},
					UiSubcommand::PaneRatioSet { ratio } => GlorpCommand::Ui(UiCommand::PaneRatioSet { ratio }),
				};
				output::print_outcome(&host.execute(command)?)?;
			}
			Command::Scene { command } => match command {
				SceneSubcommand::Ensure => {
					output::print_outcome(&host.execute(GlorpCommand::Scene(SceneCommand::Ensure))?)?;
				}
			},
			Command::Txn { json } => {
				let txn: GlorpTxn = serde_json::from_str(&json)
					.map_err(|error| GlorpError::validation(None, format!("invalid txn JSON: {error}")))?;
				output::print_outcome(&host.execute(GlorpCommand::Txn(txn))?)?;
			}
		}

		Ok(())
	}

	fn host(&self) -> Result<Host, GlorpError> {
		if let Some(socket) = self
			.socket
			.clone()
			.or_else(|| std::env::var_os("GLORP_SOCKET").map(PathBuf::from))
		{
			return Ok(Host::Ipc(IpcClient::new(socket)));
		}

		let repo_root = self.repo_root.clone().unwrap_or(
			std::env::current_dir()
				.map_err(|error| GlorpError::transport(format!("failed to determine current directory: {error}")))?,
		);
		if let Some(socket) = autodetect_socket(&repo_root) {
			return Ok(Host::Ipc(IpcClient::new(socket)));
		}
		let options = RuntimeOptions {
			paths: default_runtime_paths(repo_root),
		};
		Ok(Host::Local(RuntimeHost::new(options)?))
	}
}

fn autodetect_socket(repo_root: &Path) -> Option<PathBuf> {
	let socket = default_socket_path(repo_root);
	socket_is_live(&socket).then_some(socket)
}

impl GlorpHost for Host {
	fn execute(&mut self, command: GlorpCommand) -> Result<GlorpOutcome, GlorpError> {
		match self {
			Host::Local(host) => host.execute(command),
			Host::Ipc(host) => host.execute(command),
		}
	}

	fn query(&mut self, query: GlorpQuery) -> Result<GlorpQueryResult, GlorpError> {
		match self {
			Host::Local(host) => host.query(query),
			Host::Ipc(host) => host.query(query),
		}
	}

	fn subscribe(&mut self, request: GlorpSubscription) -> Result<GlorpStreamToken, GlorpError> {
		match self {
			Host::Local(host) => host.subscribe(request),
			Host::Ipc(host) => host.subscribe(request),
		}
	}

	fn next_event(&mut self, token: GlorpStreamToken) -> Result<Option<GlorpEvent>, GlorpError> {
		match self {
			Host::Local(host) => host.next_event(token),
			Host::Ipc(host) => host.next_event(token),
		}
	}

	fn unsubscribe(&mut self, token: GlorpStreamToken) -> Result<(), GlorpError> {
		match self {
			Host::Local(host) => host.unsubscribe(token),
			Host::Ipc(host) => host.unsubscribe(token),
		}
	}
}

fn parse_value(input: &str) -> GlorpValue {
	serde_json::from_str::<serde_json::Value>(input)
		.map(GlorpValue::from)
		.unwrap_or_else(|_| GlorpValue::String(input.to_owned()))
}

fn flatten_patch(prefix: Option<&str>, value: &GlorpValue) -> Result<Vec<ConfigAssignment>, GlorpError> {
	match value {
		GlorpValue::Record(fields) => {
			let mut assignments = Vec::new();
			for (key, value) in fields {
				let path = prefix
					.map(|prefix| format!("{prefix}.{key}"))
					.unwrap_or_else(|| key.clone());
				assignments.extend(flatten_patch(Some(&path), value)?);
			}
			Ok(assignments)
		}
		other => Ok(vec![ConfigAssignment {
			path: prefix.unwrap_or_default().to_owned(),
			value: other.clone(),
		}]),
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
