use {
	crate::{TransportRequest, TransportResponse},
	glorp_api::*,
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
	pub fn socket_path(&self) -> &Path {
		&self.socket_path
	}

	pub fn shutdown(mut self) -> Result<(), GlorpError> {
		self.stop.store(true, Ordering::SeqCst);
		let _ = super::transport_request::<TransportResponse>(&self.socket_path, &TransportRequest::Shutdown);
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
					thread::spawn(move || {
						let _ = handle_connection(stream, &host);
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

fn handle_connection(stream: UnixStream, host: &Arc<Mutex<RuntimeHost>>) -> Result<(), GlorpError> {
	let mut line = String::new();
	let mut reader = BufReader::new(
		stream
			.try_clone()
			.map_err(|error| GlorpError::transport(format!("failed to clone stream: {error}")))?,
	);
	reader
		.read_line(&mut line)
		.map_err(|error| GlorpError::transport(format!("failed to read request: {error}")))?;
	let request: TransportRequest = serde_json::from_str(&line)
		.map_err(|error| GlorpError::internal(format!("failed to decode request: {error}")))?;

	let response = {
		let mut host = host
			.lock()
			.map_err(|_| GlorpError::transport("runtime lock poisoned"))?;
		match request {
			TransportRequest::Execute(command) => TransportResponse::Execute(host.execute(command)),
			TransportRequest::Query(query) => TransportResponse::Query(Box::new(host.query(query))),
			TransportRequest::Subscribe(request) => TransportResponse::Subscribe(host.subscribe(request)),
			TransportRequest::NextEvent(token) => TransportResponse::NextEvent(host.next_event(token)),
			TransportRequest::Unsubscribe(token) => TransportResponse::Unsubscribe(host.unsubscribe(token)),
			TransportRequest::Shutdown => TransportResponse::Shutdown(Ok(())),
		}
	};

	let payload = serde_json::to_string(&response)
		.map_err(|error| GlorpError::internal(format!("failed to encode response: {error}")))?;
	let mut writer = stream;
	writer
		.write_all(payload.as_bytes())
		.and_then(|()| writer.write_all(b"\n"))
		.map_err(|error| GlorpError::transport(format!("failed to write response: {error}")))
}
