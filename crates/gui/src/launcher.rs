use {
	glorp_api::{GlorpError, GlorpHost, GlorpQuery, GlorpQueryResult},
	glorp_runtime::{RuntimeHost, RuntimeOptions, default_runtime_paths},
	glorp_transport::{
		IpcClient, IpcServerHandle, LocalClient, default_socket_path, socket_is_live, start_server_shared,
	},
	std::{
		path::{Path, PathBuf},
		sync::{Arc, Mutex},
	},
};

#[derive(Debug, Clone)]
pub struct GuiLaunchOptions {
	pub repo_root: PathBuf,
	pub socket_path: PathBuf,
}

impl GuiLaunchOptions {
	#[must_use]
	pub fn for_repo_root(repo_root: impl Into<PathBuf>) -> Self {
		let repo_root = repo_root.into();
		Self {
			socket_path: default_socket_path(&repo_root),
			repo_root,
		}
	}
}

pub struct GuiRuntimeSession {
	socket_path: PathBuf,
	host: Option<Arc<Mutex<RuntimeHost>>>,
	server: Option<IpcServerHandle>,
}

#[derive(Clone)]
pub enum GuiRuntimeClient {
	Local(LocalClient),
	Ipc(IpcClient),
}

impl GuiRuntimeSession {
	pub fn start_owned(options: GuiLaunchOptions) -> Result<(Self, LocalClient), GlorpError> {
		if socket_is_live(&options.socket_path) {
			return Err(GlorpError::transport(format!(
				"shared GUI socket already active at {}",
				options.socket_path.display()
			)));
		}

		ensure_socket_parent(&options.socket_path)?;

		let host = Arc::new(Mutex::new(RuntimeHost::new(RuntimeOptions {
			paths: default_runtime_paths(&options.repo_root),
		})?));
		let server = start_server_shared(options.socket_path.clone(), Arc::clone(&host))?;
		let mut client = LocalClient::shared(Arc::clone(&host));
		ensure_runtime_capabilities(&mut client, "unexpected capabilities response from GUI runtime")?;

		Ok((
			Self {
				socket_path: options.socket_path,
				host: Some(host),
				server: Some(server),
			},
			client,
		))
	}

	pub fn connect_or_start(options: GuiLaunchOptions) -> Result<(Self, GuiRuntimeClient), GlorpError> {
		if socket_is_live(&options.socket_path) {
			let mut client = IpcClient::new(options.socket_path.clone());
			ensure_runtime_capabilities(&mut client, "unexpected capabilities response from shared GUI runtime")?;

			Ok((
				Self {
					socket_path: options.socket_path,
					host: None,
					server: None,
				},
				GuiRuntimeClient::Ipc(client),
			))
		} else {
			let (session, client) = Self::start_owned(options)?;
			Ok((session, GuiRuntimeClient::Local(client)))
		}
	}

	#[must_use]
	pub fn socket_path(&self) -> &Path {
		&self.socket_path
	}

	#[must_use]
	pub fn host(&self) -> Arc<Mutex<RuntimeHost>> {
		Arc::clone(
			self.host
				.as_ref()
				.expect("GUI runtime session does not own a local host"),
		)
	}

	#[must_use]
	pub const fn owns_server(&self) -> bool {
		self.server.is_some()
	}

	pub fn shutdown(&mut self) -> Result<(), GlorpError> {
		if let Some(server) = self.server.take() {
			server.shutdown()?;
		}
		Ok(())
	}
}

fn ensure_socket_parent(socket_path: &Path) -> Result<(), GlorpError> {
	let Some(parent) = socket_path.parent() else {
		return Ok(());
	};

	std::fs::create_dir_all(parent)
		.map_err(|error| GlorpError::transport(format!("failed to create socket parent {}: {error}", parent.display())))
}

fn ensure_runtime_capabilities(client: &mut impl GlorpHost, error: &'static str) -> Result<(), GlorpError> {
	let GlorpQueryResult::Capabilities(_) = client.query(GlorpQuery::Capabilities)? else {
		return Err(GlorpError::transport(error));
	};

	Ok(())
}

impl GuiRuntimeClient {
	fn as_glorp_host(&mut self) -> &mut dyn GlorpHost {
		match self {
			Self::Local(client) => client,
			Self::Ipc(client) => client,
		}
	}
}

impl GlorpHost for GuiRuntimeClient {
	fn execute(&mut self, command: glorp_api::GlorpCommand) -> Result<glorp_api::GlorpOutcome, GlorpError> {
		self.as_glorp_host().execute(command)
	}

	fn query(&mut self, query: GlorpQuery) -> Result<GlorpQueryResult, GlorpError> {
		self.as_glorp_host().query(query)
	}

	fn subscribe(&mut self, request: glorp_api::GlorpSubscription) -> Result<glorp_api::GlorpStreamToken, GlorpError> {
		self.as_glorp_host().subscribe(request)
	}

	fn next_event(&mut self, token: glorp_api::GlorpStreamToken) -> Result<Option<glorp_api::GlorpEvent>, GlorpError> {
		self.as_glorp_host().next_event(token)
	}

	fn unsubscribe(&mut self, token: glorp_api::GlorpStreamToken) -> Result<(), GlorpError> {
		self.as_glorp_host().unsubscribe(token)
	}
}

impl Drop for GuiRuntimeSession {
	fn drop(&mut self) {
		let _ = self.shutdown();
	}
}
