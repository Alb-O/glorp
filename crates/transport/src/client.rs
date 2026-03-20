use {
	crate::{
		GuiSessionOpen, GuiTransportRequest, GuiTransportResponse, ServerRequest, ServerResponse, TransportRequest,
		TransportResponse,
	},
	glorp_api::{GlorpCall, GlorpCallDescriptor, GlorpCallResult, GlorpCaller, GlorpError},
	glorp_editor::ScenePresentation,
	glorp_runtime::{
		GuiEditRequest, GuiEditResponse, GuiLayoutRequest, GuiRuntimeFrame, GuiSessionClientMessage,
		GuiSessionHostMessage, GuiSessionRequest, GuiSessionResponse,
	},
	serde::{Serialize, de::DeserializeOwned},
	std::{
		collections::BTreeMap,
		io::{BufRead, BufReader, Write},
		os::unix::net::UnixStream,
		path::{Path, PathBuf},
		sync::{
			Arc, Mutex,
			atomic::{AtomicU64, Ordering},
			mpsc,
		},
		thread,
	},
};

#[derive(Debug, Clone)]
pub struct IpcClient {
	socket_path: PathBuf,
}

pub struct GuiSessionClient {
	socket_path: PathBuf,
	writer: Arc<Mutex<UnixStream>>,
	next_id: AtomicU64,
	pending: Arc<Mutex<BTreeMap<u64, mpsc::Sender<Result<GuiSessionResponse, GlorpError>>>>>,
}

impl IpcClient {
	pub fn new(socket_path: impl Into<PathBuf>) -> Self {
		Self {
			socket_path: socket_path.into(),
		}
	}
}

impl GuiSessionClient {
	pub fn connect(
		socket_path: impl Into<PathBuf>, layout: GuiLayoutRequest,
	) -> Result<(Self, GuiRuntimeFrame, mpsc::Receiver<GuiSessionHostMessage>), GlorpError> {
		let socket_path = socket_path.into();
		let mut stream = UnixStream::connect(&socket_path).map_err(|error| {
			GlorpError::transport(format!("failed to connect to {}: {error}", socket_path.display()))
		})?;
		write_json(&mut stream, &ServerRequest::GuiSessionOpen(GuiSessionOpen { layout }))?;
		let ready = read_json::<ServerResponse>(&stream)?;
		let ServerResponse::GuiSessionReady(GuiSessionHostMessage::Ready { frame }) = ready else {
			return Err(GlorpError::transport("unexpected gui session handshake response"));
		};

		let reader = stream
			.try_clone()
			.map_err(|error| GlorpError::transport(format!("failed to clone gui session socket: {error}")))?;
		let writer = Arc::new(Mutex::new(stream));
		let pending = Arc::new(Mutex::new(BTreeMap::new()));
		let (events_tx, events_rx) = mpsc::channel();
		spawn_gui_session_reader(reader, Arc::clone(&pending), events_tx);

		Ok((
			Self {
				socket_path,
				writer,
				next_id: AtomicU64::new(1),
				pending,
			},
			*frame,
			events_rx,
		))
	}

	pub fn gui_edit(&self, request: GuiEditRequest) -> Result<GuiEditResponse, GlorpError> {
		match self.request(GuiSessionRequest::Edit(request))? {
			GuiSessionResponse::Edit(result) => result,
			_ => Err(unexpected_response("gui session edit")),
		}
	}

	pub fn gui_frame(&self, layout: GuiLayoutRequest) -> Result<GuiRuntimeFrame, GlorpError> {
		match self.request(GuiSessionRequest::GuiFrame(layout))? {
			GuiSessionResponse::GuiFrame(result) => result,
			_ => Err(unexpected_response("gui session frame")),
		}
	}

	pub fn scene_fetch(&self, layout: GuiLayoutRequest) -> Result<ScenePresentation, GlorpError> {
		match self.request(GuiSessionRequest::SceneFetch(layout))? {
			GuiSessionResponse::SceneFetch(result) => result,
			_ => Err(unexpected_response("gui session scene fetch")),
		}
	}

	fn request(&self, body: GuiSessionRequest) -> Result<GuiSessionResponse, GlorpError> {
		let id = self.next_id.fetch_add(1, Ordering::SeqCst);
		let (reply_tx, reply_rx) = mpsc::channel();
		self.pending
			.lock()
			.expect("gui session pending reply map should not be poisoned")
			.insert(id, reply_tx);
		let send_result = {
			let mut writer = self
				.writer
				.lock()
				.expect("gui session writer lock should not be poisoned");
			write_json(
				&mut *writer,
				&ServerRequest::GuiSessionMessage(GuiSessionClientMessage::Request { id, body }),
			)
		};
		if let Err(error) = send_result {
			self.pending
				.lock()
				.expect("gui session pending reply map should not be poisoned")
				.remove(&id);
			return Err(error);
		}

		reply_rx.recv().map_err(|_| {
			GlorpError::transport(format!(
				"gui session closed while waiting for reply from {}",
				self.socket_path.display()
			))
		})?
	}
}

#[must_use]
pub fn socket_is_live(socket_path: &Path) -> bool {
	socket_path.exists() && {
		let mut client = IpcClient::new(socket_path);
		glorp_api::calls::Capabilities::call(&mut client, ()).is_ok()
	}
}

pub fn transport_request(socket_path: &Path, request: TransportRequest) -> Result<TransportResponse, GlorpError> {
	expect_response(
		request_response(socket_path, ServerRequest::Public(request)),
		"public transport",
		|response| match response {
			ServerResponse::Public(response) => Some(response),
			ServerResponse::Gui(_) | ServerResponse::GuiSessionReady(_) | ServerResponse::GuiSessionMessage(_) => None,
		},
	)
}

pub fn gui_transport_request(
	socket_path: &Path, request: GuiTransportRequest,
) -> Result<GuiTransportResponse, GlorpError> {
	expect_response(
		request_response(socket_path, ServerRequest::Gui(request)),
		"private gui transport",
		|response| match response {
			ServerResponse::Gui(response) => Some(response),
			ServerResponse::Public(_) | ServerResponse::GuiSessionReady(_) | ServerResponse::GuiSessionMessage(_) => {
				None
			}
		},
	)
}

fn request_response<Req, Resp>(socket_path: &Path, request: Req) -> Result<Resp, GlorpError>
where
	Req: Serialize,
	Resp: DeserializeOwned, {
	let mut stream = UnixStream::connect(socket_path)
		.map_err(|error| GlorpError::transport(format!("failed to connect to {}: {error}", socket_path.display())))?;
	write_json(&mut stream, &request)?;
	read_json(&stream)
}

impl GlorpCaller for IpcClient {
	fn call(&mut self, call: GlorpCall) -> Result<GlorpCallResult, GlorpError> {
		expect_response(
			self.response(TransportRequest::Call(call)),
			"call",
			|response| match response {
				TransportResponse::Call(result) => Some(*result),
			},
		)?
	}
}

impl GlorpCaller for GuiSessionClient {
	fn call(&mut self, call: GlorpCall) -> Result<GlorpCallResult, GlorpError> {
		match self.request(GuiSessionRequest::Call(call))? {
			GuiSessionResponse::Call(result) => result,
			_ => Err(unexpected_response("gui session call")),
		}
	}
}

impl IpcClient {
	fn response(&self, request: TransportRequest) -> Result<TransportResponse, GlorpError> {
		transport_request(&self.socket_path, request)
	}
}

fn spawn_gui_session_reader(
	reader: UnixStream, pending: Arc<Mutex<BTreeMap<u64, mpsc::Sender<Result<GuiSessionResponse, GlorpError>>>>>,
	events_tx: mpsc::Sender<GuiSessionHostMessage>,
) {
	thread::spawn(move || {
		let mut lines = BufReader::new(reader);
		loop {
			let mut line = String::new();
			match lines.read_line(&mut line) {
				Ok(0) => break,
				Ok(_) => {}
				Err(_) => break,
			}
			let Ok(response) = serde_json::from_str::<ServerResponse>(&line) else {
				break;
			};
			match response {
				ServerResponse::GuiSessionMessage(GuiSessionHostMessage::Reply { id, body }) => {
					if let Some(reply_tx) = pending
						.lock()
						.expect("gui session pending reply map should not be poisoned")
						.remove(&id)
					{
						let _ = reply_tx.send(Ok(body));
					}
				}
				ServerResponse::GuiSessionMessage(message @ GuiSessionHostMessage::Changed(_))
				| ServerResponse::GuiSessionMessage(message @ GuiSessionHostMessage::Closed) => {
					let _ = events_tx.send(message);
				}
				ServerResponse::GuiSessionReady(_) => {}
				ServerResponse::Public(_) | ServerResponse::Gui(_) => break,
				ServerResponse::GuiSessionMessage(GuiSessionHostMessage::Ready { .. }) => {}
			}
		}

		let waiters = pending
			.lock()
			.expect("gui session pending reply map should not be poisoned")
			.split_off(&0);
		for (_, reply_tx) in waiters {
			let _ = reply_tx.send(Err(GlorpError::transport("gui session reader stopped")));
		}
		let _ = events_tx.send(GuiSessionHostMessage::Closed);
	});
}

fn write_json(stream: &mut UnixStream, request: &impl Serialize) -> Result<(), GlorpError> {
	let payload = serde_json::to_string(request)
		.map_err(|error| GlorpError::internal(format!("failed to encode request: {error}")))?;
	writeln!(stream, "{payload}").map_err(|error| GlorpError::transport(format!("failed to write request: {error}")))
}

fn read_json<Response: DeserializeOwned>(stream: &UnixStream) -> Result<Response, GlorpError> {
	let mut response = String::new();
	BufReader::new(stream)
		.read_line(&mut response)
		.map_err(|error| GlorpError::transport(format!("failed to read response: {error}")))?;
	serde_json::from_str(&response).map_err(|error| GlorpError::internal(format!("failed to decode response: {error}")))
}

fn expect_response<T, Response>(
	response: Result<Response, GlorpError>, kind: &str, extract: impl FnOnce(Response) -> Option<T>,
) -> Result<T, GlorpError> {
	extract(response?).ok_or_else(|| unexpected_response(kind))
}

fn unexpected_response(kind: &str) -> GlorpError {
	GlorpError::transport(format!("unexpected {kind} response"))
}
