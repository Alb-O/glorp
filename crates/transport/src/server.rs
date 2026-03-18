use {
	crate::{
		GuiTransportRequest, GuiTransportResponse, ServerRequest, ServerResponse, TransportRequest, TransportResponse,
	},
	glorp_api::{GlorpError, GlorpHost},
	glorp_runtime::RuntimeHost,
	std::{
		io::{BufRead, BufReader, Write},
		os::unix::net::{UnixListener, UnixStream},
		path::{Path, PathBuf},
		sync::{
			Arc, Mutex,
			atomic::{AtomicBool, Ordering},
		},
		thread::{self, JoinHandle},
		time::Duration,
	},
};

pub struct IpcServerHandle {
	socket_path: PathBuf,
	stop: Arc<AtomicBool>,
	thread: Option<JoinHandle<()>>,
}

impl IpcServerHandle {
	#[must_use]
	pub fn socket_path(&self) -> &Path {
		&self.socket_path
	}

	pub fn shutdown(mut self) -> Result<(), GlorpError> {
		self.stop.store(true, Ordering::SeqCst);
		let _ = super::transport_request(&self.socket_path, TransportRequest::Shutdown);
		self.join()
	}

	pub fn wait(mut self) -> Result<(), GlorpError> {
		self.join()
	}

	fn join(&mut self) -> Result<(), GlorpError> {
		if let Some(thread) = self.thread.take() {
			thread
				.join()
				.map_err(|_| GlorpError::transport("failed to join IPC server thread"))?;
		}
		let _ = std::fs::remove_file(&self.socket_path);
		Ok(())
	}
}

pub fn start_server(socket_path: impl Into<PathBuf>, host: RuntimeHost) -> Result<IpcServerHandle, GlorpError> {
	start_server_shared(socket_path, Arc::new(Mutex::new(host)))
}

pub fn start_server_shared(
	socket_path: impl Into<PathBuf>, host: Arc<Mutex<RuntimeHost>>,
) -> Result<IpcServerHandle, GlorpError> {
	let socket_path = socket_path.into();
	if socket_path.exists() {
		std::fs::remove_file(&socket_path)
			.map_err(|error| GlorpError::transport(format!("failed to remove stale socket: {error}")))?;
	}

	let listener = UnixListener::bind(&socket_path)
		.map_err(|error| GlorpError::transport(format!("failed to bind {}: {error}", socket_path.display())))?;
	listener
		.set_nonblocking(true)
		.map_err(|error| GlorpError::transport(format!("failed to mark socket nonblocking: {error}")))?;

	let stop = Arc::new(AtomicBool::new(false));
	let stop_thread = Arc::clone(&stop);
	let socket_path_thread = socket_path.clone();
	let thread = thread::spawn(move || {
		while !stop_thread.load(Ordering::SeqCst) {
			match listener.accept() {
				Ok((stream, _)) => {
					let host = Arc::clone(&host);
					let stop = Arc::clone(&stop_thread);
					thread::spawn(move || {
						let _ = handle_connection(stream, &host, &stop);
					});
				}
				Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
					thread::sleep(Duration::from_millis(10));
				}
				Err(_) => break,
			}
		}
		let _ = std::fs::remove_file(socket_path_thread);
	});

	Ok(IpcServerHandle {
		socket_path,
		stop,
		thread: Some(thread),
	})
}

fn handle_connection(
	stream: UnixStream, host: &Arc<Mutex<RuntimeHost>>, stop: &Arc<AtomicBool>,
) -> Result<(), GlorpError> {
	let mut stream = stream;
	let request = read_request(&stream)?;
	let response = dispatch_request(request, host, stop)?;

	let payload = serde_json::to_string(&response)
		.map_err(|error| GlorpError::internal(format!("failed to encode response: {error}")))?;
	writeln!(stream, "{payload}").map_err(|error| GlorpError::transport(format!("failed to write response: {error}")))
}

fn read_request(stream: &UnixStream) -> Result<ServerRequest, GlorpError> {
	let mut line = String::new();
	let mut reader = BufReader::new(stream);
	reader
		.read_line(&mut line)
		.map_err(|error| GlorpError::transport(format!("failed to read request: {error}")))?;
	serde_json::from_str(&line).map_err(|error| GlorpError::internal(format!("failed to decode request: {error}")))
}

fn dispatch_request(
	request: ServerRequest, host: &Arc<Mutex<RuntimeHost>>, stop: &Arc<AtomicBool>,
) -> Result<ServerResponse, GlorpError> {
	let mut host = host
		.lock()
		.map_err(|_| GlorpError::transport("runtime lock poisoned"))?;
	Ok(match request {
		ServerRequest::Public(request) => ServerResponse::Public(dispatch_public_request(request, &mut host, stop)),
		ServerRequest::Gui(request) => ServerResponse::Gui(dispatch_gui_request(request, &mut host)),
	})
}

fn dispatch_public_request(
	request: TransportRequest, host: &mut RuntimeHost, stop: &Arc<AtomicBool>,
) -> TransportResponse {
	match request {
		TransportRequest::Execute(command) => TransportResponse::Execute(host.execute(command)),
		TransportRequest::Query(query) => TransportResponse::Query(Box::new(host.query(query))),
		TransportRequest::Subscribe(request) => TransportResponse::Subscribe(host.subscribe(request)),
		TransportRequest::NextEvent(token) => TransportResponse::NextEvent(host.next_event(token)),
		TransportRequest::Unsubscribe(token) => TransportResponse::Unsubscribe(host.unsubscribe(token)),
		TransportRequest::Shutdown => {
			stop.store(true, Ordering::SeqCst);
			TransportResponse::Shutdown(Ok(()))
		}
	}
}

fn dispatch_gui_request(request: GuiTransportRequest, host: &mut RuntimeHost) -> GuiTransportResponse {
	match request {
		GuiTransportRequest::ExecuteGui(command) => GuiTransportResponse::ExecuteGui(host.execute_gui(command)),
		GuiTransportRequest::GuiFrame => GuiTransportResponse::GuiFrame(Box::new(Ok(host.gui_transport_frame()))),
	}
}
