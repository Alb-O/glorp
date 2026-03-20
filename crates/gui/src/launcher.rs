use {
	glorp_api::{GlorpCall, GlorpCallDescriptor, GlorpCallResult, GlorpCaller, GlorpError},
	glorp_runtime::{
		DEFAULT_LAYOUT_WIDTH, GuiCommand, GuiEditRequest, GuiEditResponse, GuiLayoutRequest, GuiRuntimeFrame,
		RuntimeHost, RuntimeOptions, default_runtime_paths,
	},
	glorp_transport::{
		GuiTransportRequest, GuiTransportResponse, IpcClient, IpcServerHandle, LocalClient, default_socket_path,
		ensure_socket_parent, gui_transport_request, socket_is_live, start_server_shared, wait_for_socket,
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
	server: Option<IpcServerHandle>,
}

pub struct GuiRuntimeClient {
	client: RuntimeClient,
	socket_path: PathBuf,
	layout_width: f32,
}

#[derive(Clone)]
enum RuntimeClient {
	Ipc(IpcClient),
	Local(LocalClient),
}

impl GuiRuntimeSession {
	pub fn start_owned(options: GuiLaunchOptions) -> Result<(Self, GuiRuntimeClient), GlorpError> {
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
		let server = start_server_shared(options.socket_path.as_path(), Arc::clone(&host))?;
		wait_for_socket(&options.socket_path)?;

		let mut client = GuiRuntimeClient::new_local(options.socket_path.clone(), Arc::clone(&host));
		ensure_runtime_capabilities(&mut client, "unexpected capabilities response from GUI runtime")?;

		Ok((
			Self {
				socket_path: options.socket_path,
				server: Some(server),
			},
			client,
		))
	}

	pub fn connect_or_start(options: GuiLaunchOptions) -> Result<(Self, GuiRuntimeClient), GlorpError> {
		if !socket_is_live(&options.socket_path) {
			return Self::start_owned(options);
		}

		let mut client = GuiRuntimeClient::new_ipc(options.socket_path.clone());
		ensure_runtime_capabilities(&mut client, "unexpected capabilities response from shared GUI runtime")?;

		Ok((
			Self {
				socket_path: options.socket_path,
				server: None,
			},
			client,
		))
	}

	#[must_use]
	pub fn socket_path(&self) -> &Path {
		&self.socket_path
	}

	#[must_use]
	pub const fn owns_server(&self) -> bool {
		self.server.is_some()
	}

	pub fn shutdown(&mut self) -> Result<(), GlorpError> {
		self.server.take().map_or(Ok(()), IpcServerHandle::shutdown)
	}
}

impl GuiRuntimeClient {
	#[must_use]
	pub fn new_ipc(socket_path: impl Into<PathBuf>) -> Self {
		let socket_path = socket_path.into();
		Self {
			client: RuntimeClient::Ipc(IpcClient::new(socket_path.as_path())),
			socket_path,
			layout_width: DEFAULT_LAYOUT_WIDTH,
		}
	}

	#[must_use]
	pub fn new_local(socket_path: impl Into<PathBuf>, host: Arc<Mutex<RuntimeHost>>) -> Self {
		Self {
			client: RuntimeClient::Local(LocalClient::shared(host)),
			socket_path: socket_path.into(),
			layout_width: DEFAULT_LAYOUT_WIDTH,
		}
	}

	pub fn set_layout_width(&mut self, layout_width: f32) {
		self.layout_width = layout_width.max(1.0);
	}

	pub fn execute_gui(&mut self, command: GuiCommand) -> Result<(), GlorpError> {
		let layout = self.layout_request();
		match &self.client {
			RuntimeClient::Ipc(_) => {
				let GuiTransportResponse::ExecuteGui(result) =
					gui_transport_request(&self.socket_path, GuiTransportRequest::ExecuteGui { layout, command })?
				else {
					return Err(GlorpError::transport("unexpected private gui execute response"));
				};
				result
			}
			RuntimeClient::Local(client) => with_local_runtime(client, |host| host.execute_gui_at(layout, command)),
		}
	}

	pub fn gui_edit(&mut self, mut request: GuiEditRequest) -> Result<GuiEditResponse, GlorpError> {
		request.layout = self.layout_request();
		match &self.client {
			RuntimeClient::Ipc(_) => {
				let GuiTransportResponse::Edit(result) =
					gui_transport_request(&self.socket_path, GuiTransportRequest::Edit(request))?
				else {
					return Err(GlorpError::transport("unexpected private gui edit response"));
				};
				*result
			}
			RuntimeClient::Local(client) => with_local_runtime(client, |host| host.gui_edit(request)),
		}
	}

	pub fn gui_frame(&mut self) -> Result<GuiRuntimeFrame, GlorpError> {
		let layout = self.layout_request();
		match &self.client {
			RuntimeClient::Ipc(_) => {
				let GuiTransportResponse::GuiFrame(result) =
					gui_transport_request(&self.socket_path, GuiTransportRequest::GuiFrame(layout))?
				else {
					return Err(GlorpError::transport("unexpected private gui frame response"));
				};
				*result
			}
			RuntimeClient::Local(client) => with_local_runtime(client, |host| Ok(host.gui_frame_at(layout))),
		}
	}

	fn layout_request(&self) -> GuiLayoutRequest {
		GuiLayoutRequest {
			layout_width: self.layout_width,
		}
	}
}

impl GlorpCaller for GuiRuntimeClient {
	fn call(&mut self, call: GlorpCall) -> Result<GlorpCallResult, GlorpError> {
		match &mut self.client {
			RuntimeClient::Ipc(client) => client.call(call),
			RuntimeClient::Local(client) => client.call(call),
		}
	}
}

fn with_local_runtime<T>(
	client: &LocalClient, f: impl FnOnce(&mut RuntimeHost) -> Result<T, GlorpError>,
) -> Result<T, GlorpError> {
	let host = client.host();
	let mut host = host
		.lock()
		.map_err(|_| GlorpError::transport("local runtime lock poisoned"))?;
	f(&mut host)
}

fn ensure_runtime_capabilities(client: &mut impl GlorpCaller, error: &'static str) -> Result<(), GlorpError> {
	glorp_api::calls::Capabilities::call(client, ())
		.map(|_| ())
		.map_err(|_| GlorpError::transport(error))
}

impl Drop for GuiRuntimeSession {
	fn drop(&mut self) {
		let _ = self.shutdown();
	}
}
