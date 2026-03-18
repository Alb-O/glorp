use {
	crate::output,
	clap::{Parser, Subcommand},
	glorp_api::{
		GlorpCapabilities, GlorpCommand, GlorpError, GlorpEvent, GlorpEventStreamView, GlorpHost, GlorpOutcome,
		GlorpQuery, GlorpQueryResult, GlorpSessionView, GlorpStreamToken, GlorpSubscription, GlorpTxn,
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
	Session {
		#[command(subcommand)]
		command: SessionSubcommand,
	},
	Execute {
		#[arg(long)]
		json: String,
	},
	Query {
		#[arg(long)]
		json: String,
	},
	Txn {
		#[arg(long)]
		json: String,
	},
	Events {
		#[command(subcommand)]
		command: EventsSubcommand,
	},
}

#[derive(Debug, Subcommand)]
enum SessionSubcommand {
	Attach,
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
			Command::Session {
				command: SessionSubcommand::Attach,
			} => output::print_json(&self.attach_session()?)?,
			Command::Execute { json } => {
				let command = parse_json::<GlorpCommand>(&json, "command JSON")?;
				output::print_outcome(&host.execute(command)?)?;
			}
			Command::Query { json } => {
				let query = parse_json::<GlorpQuery>(&json, "query JSON")?;
				output::print_query(&host.query(query)?)?;
			}
			Command::Txn { json } => {
				let txn = parse_json::<GlorpTxn>(&json, "txn JSON")?;
				output::print_outcome(&host.execute(GlorpCommand::Txn(txn))?)?;
			}
			Command::Events { command } => run_events(host, command)?,
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

fn parse_json<T>(json: &str, label: &str) -> Result<T, GlorpError>
where
	T: serde::de::DeserializeOwned, {
	serde_json::from_str(json).map_err(|error| GlorpError::validation(None, format!("invalid {label}: {error}")))
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
