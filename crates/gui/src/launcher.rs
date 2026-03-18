use {
	glorp_api::{GlorpError, GlorpHost, GlorpQuery, GlorpQueryResult},
	glorp_runtime::{RuntimeHost, RuntimeOptions, default_runtime_paths},
	glorp_transport::{IpcClient, IpcServerHandle, default_socket_path, socket_is_live, start_server},
	std::path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub struct GuiLaunchOptions {
	pub repo_root: PathBuf,
	pub socket_path: PathBuf,
}

impl GuiLaunchOptions {
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
	server: Option<IpcServerHandle>,
}

impl GuiRuntimeSession {
	pub fn connect_or_start(options: GuiLaunchOptions) -> Result<(Self, IpcClient), GlorpError> {
		if socket_is_live(&options.socket_path) {
			let client = IpcClient::new(options.socket_path.clone());
			return Ok((
				Self {
					socket_path: options.socket_path,
					server: None,
				},
				client,
			));
		}

		if let Some(parent) = options.socket_path.parent() {
			std::fs::create_dir_all(parent).map_err(|error| {
				GlorpError::transport(format!("failed to create socket parent {}: {error}", parent.display()))
			})?;
		}

		let host = RuntimeHost::new(RuntimeOptions {
			paths: default_runtime_paths(&options.repo_root),
		})?;
		let server = start_server(options.socket_path.clone(), host)?;
		let client = IpcClient::new(options.socket_path.clone());

		match client.clone().query(GlorpQuery::Capabilities)? {
			GlorpQueryResult::Capabilities(_) => Ok((
				Self {
					socket_path: options.socket_path,
					server: Some(server),
				},
				client,
			)),
			_ => Err(GlorpError::transport(
				"unexpected capabilities response from GUI runtime",
			)),
		}
	}

	pub fn socket_path(&self) -> &Path {
		&self.socket_path
	}

	pub fn owns_server(&self) -> bool {
		self.server.is_some()
	}

	pub fn shutdown(&mut self) -> Result<(), GlorpError> {
		if let Some(server) = self.server.take() {
			server.shutdown()?;
		}
		Ok(())
	}
}

impl Drop for GuiRuntimeSession {
	fn drop(&mut self) {
		let _ = self.shutdown();
	}
}
