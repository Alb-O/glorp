use {
	crate::{
		GuiTransportRequest, GuiTransportResponse, ServerRequest, ServerResponse, TransportRequest, TransportResponse,
	},
	glorp_api::{
		GlorpCall, GlorpCallDescriptor, GlorpCallResult, GlorpCallRoute, GlorpCaller, GlorpError, OkView,
		TransportCallDispatcher, call_spec, dispatch_transport_call,
	},
	glorp_runtime::{
		GuiSessionClientMessage, GuiSessionHostMessage, GuiSessionRequest, GuiSessionResponse, RuntimeHost,
	},
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
		let _ = super::transport_request(
			&self.socket_path,
			TransportRequest::Call(
				glorp_api::calls::SessionShutdown::build(()).expect("session-shutdown should build"),
			),
		);
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
	let request = read_request(&stream)?;
	match request {
		ServerRequest::GuiSessionOpen(open) => handle_gui_session(stream, host, stop, open.layout),
		request => {
			let response = dispatch_request(request, host, stop)?;
			write_response(&stream, &response)
		}
	}
}

fn handle_gui_session(
	stream: UnixStream, host: &Arc<Mutex<RuntimeHost>>, stop: &Arc<AtomicBool>, layout: glorp_runtime::GuiLayoutRequest,
) -> Result<(), GlorpError> {
	let ready = {
		let mut host = host
			.lock()
			.map_err(|_| GlorpError::transport("runtime lock poisoned"))?;
		ServerResponse::GuiSessionReady(GuiSessionHostMessage::Ready {
			frame: Box::new(host.gui_frame_at(layout)),
		})
	};
	write_response(&stream, &ready)?;

	let writer = Arc::new(Mutex::new(stream.try_clone().map_err(|error| {
		GlorpError::transport(format!("failed to clone gui session socket: {error}"))
	})?));
	let closed = Arc::new(AtomicBool::new(false));
	let subscriptions = {
		let host = host
			.lock()
			.map_err(|_| GlorpError::transport("runtime lock poisoned"))?;
		host.subscriptions().clone()
	};
	let token = subscriptions.subscribe(glorp_api::GlorpSubscription::Changes);
	let host_for_events = Arc::clone(host);
	let stop_for_events = Arc::clone(stop);
	let closed_for_events = Arc::clone(&closed);
	let writer_for_events = Arc::clone(&writer);
	let subscriptions_for_events = subscriptions.clone();
	let events_thread = thread::spawn(move || {
		while !stop_for_events.load(Ordering::SeqCst) && !closed_for_events.load(Ordering::SeqCst) {
			match subscriptions_for_events.next_event_blocking(token, Duration::from_millis(100)) {
				Ok(Some(glorp_api::GlorpEvent::Changed(outcome))) => {
					let response = {
						let host = match host_for_events.lock() {
							Ok(host) => host,
							Err(_) => break,
						};
						ServerResponse::GuiSessionMessage(GuiSessionHostMessage::Changed(
							host.gui_shared_delta(outcome),
						))
					};
					let Ok(writer) = writer_for_events.lock() else {
						break;
					};
					if write_response(&writer, &response).is_err() {
						break;
					}
				}
				Ok(Some(_)) => {}
				Ok(None) => {}
				Err(_) => break,
			}
		}
		let _ = subscriptions_for_events.unsubscribe(token);
	});

	let mut reader = BufReader::new(stream);
	loop {
		if stop.load(Ordering::SeqCst) {
			break;
		}
		let mut line = String::new();
		match reader.read_line(&mut line) {
			Ok(0) => break,
			Ok(_) => {}
			Err(error) => {
				return Err(GlorpError::transport(format!(
					"failed to read gui session request: {error}"
				)));
			}
		}
		let request = serde_json::from_str::<ServerRequest>(&line)
			.map_err(|error| GlorpError::internal(format!("failed to decode request: {error}")))?;
		let ServerRequest::GuiSessionMessage(GuiSessionClientMessage::Request { id, body }) = request else {
			return Err(GlorpError::transport("unexpected gui session request"));
		};
		let reply = {
			let mut host = host
				.lock()
				.map_err(|_| GlorpError::transport("runtime lock poisoned"))?;
			GuiSessionHostMessage::Reply {
				id,
				body: dispatch_gui_session_request(body, &mut host, stop),
			}
		};
		let writer = writer
			.lock()
			.map_err(|_| GlorpError::transport("gui session writer lock poisoned"))?;
		write_response(&writer, &ServerResponse::GuiSessionMessage(reply))?;
	}

	closed.store(true, Ordering::SeqCst);
	let _ = subscriptions.unsubscribe(token);
	let _ = events_thread.join();
	if let Ok(writer) = writer.lock() {
		let _ = write_response(
			&writer,
			&ServerResponse::GuiSessionMessage(GuiSessionHostMessage::Closed),
		);
	}
	Ok(())
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
		ServerRequest::GuiSessionOpen(_) | ServerRequest::GuiSessionMessage(_) => {
			return Err(GlorpError::transport(
				"gui session requests require a persistent handler",
			));
		}
	})
}

fn dispatch_public_request(
	request: TransportRequest, host: &mut RuntimeHost, stop: &Arc<AtomicBool>,
) -> TransportResponse {
	match request {
		TransportRequest::Call(call) => TransportResponse::Call(Box::new(dispatch_public_call(call, host, stop))),
	}
}

fn dispatch_gui_request(request: GuiTransportRequest, host: &mut RuntimeHost) -> GuiTransportResponse {
	match request {
		GuiTransportRequest::Edit(request) => GuiTransportResponse::Edit(Box::new(host.gui_edit(request))),
		GuiTransportRequest::GuiFrame(layout) => {
			GuiTransportResponse::GuiFrame(Box::new(Ok(host.gui_frame_at(layout))))
		}
		GuiTransportRequest::SceneFetch(layout) => {
			GuiTransportResponse::SceneFetch(Box::new(Ok(host.gui_scene_fetch_at(layout))))
		}
	}
}

fn dispatch_gui_session_request(
	request: GuiSessionRequest, host: &mut RuntimeHost, stop: &Arc<AtomicBool>,
) -> GuiSessionResponse {
	match request {
		GuiSessionRequest::Call(call) => GuiSessionResponse::Call(dispatch_public_call(call, host, stop)),
		GuiSessionRequest::Edit(request) => GuiSessionResponse::Edit(host.gui_edit(request)),
		GuiSessionRequest::GuiFrame(layout) => GuiSessionResponse::GuiFrame(Ok(host.gui_frame_at(layout))),
		GuiSessionRequest::SceneFetch(layout) => GuiSessionResponse::SceneFetch(Ok(host.gui_scene_fetch_at(layout))),
	}
}

fn dispatch_public_call(
	call: GlorpCall, host: &mut RuntimeHost, stop: &Arc<AtomicBool>,
) -> Result<GlorpCallResult, GlorpError> {
	let Some(spec) = call_spec(&call.id) else {
		return Err(GlorpError::not_found(format!("unknown call `{}`", call.id)));
	};

	match spec.route {
		GlorpCallRoute::Runtime => host.call(call),
		GlorpCallRoute::Transport => dispatch_transport_call(&mut ServerTransportDispatcher { stop }, call),
		GlorpCallRoute::Client => Err(GlorpError::validation(
			None,
			format!("call `{}` must be handled by the client route", spec.id),
		)),
	}
}

fn write_response(stream: &UnixStream, response: &ServerResponse) -> Result<(), GlorpError> {
	let payload = serde_json::to_string(response)
		.map_err(|error| GlorpError::internal(format!("failed to encode response: {error}")))?;
	let mut stream = stream;
	writeln!(stream, "{payload}").map_err(|error| GlorpError::transport(format!("failed to write response: {error}")))
}

struct ServerTransportDispatcher<'a> {
	stop: &'a Arc<AtomicBool>,
}

impl TransportCallDispatcher for ServerTransportDispatcher<'_> {
	fn session_shutdown(&mut self, _input: ()) -> Result<OkView, GlorpError> {
		self.stop.store(true, Ordering::SeqCst);
		Ok(OkView { ok: true })
	}
}
