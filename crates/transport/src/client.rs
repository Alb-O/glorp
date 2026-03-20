use {
	crate::{
		GuiSessionOpen, GuiTransportRequest, GuiTransportResponse, ServerRequest, ServerResponse, TransportRequest,
		TransportResponse,
		ipc::{
			GuiPayloadKind, GuiSessionFrame, gui_document_request, gui_scene_request, read_session_frame,
			write_session_control_frame,
		},
	},
	glorp_api::{GlorpCall, GlorpCallDescriptor, GlorpCallResult, GlorpCaller, GlorpError},
	glorp_runtime::{
		GuiDocumentFetchResponse, GuiEditRequest, GuiEditResponse, GuiLayoutRequest, GuiRuntimeFrame, GuiSceneFetchRef,
		GuiSceneFetchResponse, GuiSessionClientMessage, GuiSessionHostMessage, GuiSessionRequest, GuiSessionResponse,
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
	pending: Arc<Mutex<BTreeMap<u64, PendingReply>>>,
}

enum PendingReply {
	Control(mpsc::Sender<Result<GuiSessionResponse, GlorpError>>),
	DocumentFetch {
		sender: mpsc::Sender<Result<(GuiDocumentFetchResponse, Vec<u8>), GlorpError>>,
		response: Option<GuiDocumentFetchResponse>,
	},
	SceneFetch {
		sender: mpsc::Sender<Result<Option<(GuiSceneFetchRef, Vec<u8>)>, GlorpError>>,
		response: Option<GuiSceneFetchRef>,
	},
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

		let client = Self {
			socket_path,
			writer,
			next_id: AtomicU64::new(1),
			pending,
		};
		let frame = client.hydrate_frame(*frame)?;

		Ok((client, frame, events_rx))
	}

	pub fn gui_edit(&self, request: GuiEditRequest) -> Result<GuiEditResponse, GlorpError> {
		match self.request_control(GuiSessionRequest::Edit(request))? {
			GuiSessionResponse::Edit(result) => result,
			_ => Err(unexpected_response("gui session edit")),
		}
	}

	pub fn gui_frame(&self, layout: GuiLayoutRequest) -> Result<GuiRuntimeFrame, GlorpError> {
		match self.request_control(GuiSessionRequest::GuiFrame(layout))? {
			GuiSessionResponse::GuiFrame(result) => self.hydrate_frame(result?),
			_ => Err(unexpected_response("gui session frame")),
		}
	}

	pub fn document_fetch(&self, revision: u64) -> Result<(GuiDocumentFetchResponse, Vec<u8>), GlorpError> {
		let id = self.next_id.fetch_add(1, Ordering::SeqCst);
		let (reply_tx, reply_rx) = mpsc::channel();
		self.pending
			.lock()
			.expect("gui session pending reply map should not be poisoned")
			.insert(
				id,
				PendingReply::DocumentFetch {
					sender: reply_tx,
					response: None,
				},
			);
		self.send_control(id, gui_document_request(revision))?;
		reply_rx.recv().map_err(|_| {
			GlorpError::transport(format!(
				"gui session closed while waiting for document fetch from {}",
				self.socket_path.display()
			))
		})?
	}

	pub fn scene_fetch(
		&self, layout: GuiLayoutRequest, scene_revision: u64,
	) -> Result<Option<(GuiSceneFetchRef, Vec<u8>)>, GlorpError> {
		let id = self.next_id.fetch_add(1, Ordering::SeqCst);
		let (reply_tx, reply_rx) = mpsc::channel();
		self.pending
			.lock()
			.expect("gui session pending reply map should not be poisoned")
			.insert(
				id,
				PendingReply::SceneFetch {
					sender: reply_tx,
					response: None,
				},
			);
		self.send_control(id, gui_scene_request(layout, scene_revision))?;
		reply_rx.recv().map_err(|_| {
			GlorpError::transport(format!(
				"gui session closed while waiting for scene fetch from {}",
				self.socket_path.display()
			))
		})?
	}

	fn request_control(&self, body: GuiSessionRequest) -> Result<GuiSessionResponse, GlorpError> {
		let id = self.next_id.fetch_add(1, Ordering::SeqCst);
		let (reply_tx, reply_rx) = mpsc::channel();
		self.pending
			.lock()
			.expect("gui session pending reply map should not be poisoned")
			.insert(id, PendingReply::Control(reply_tx));
		self.send_control(id, body)?;
		reply_rx.recv().map_err(|_| {
			GlorpError::transport(format!(
				"gui session closed while waiting for reply from {}",
				self.socket_path.display()
			))
		})?
	}

	fn send_control(&self, id: u64, body: GuiSessionRequest) -> Result<(), GlorpError> {
		let send_result = {
			let mut writer = self
				.writer
				.lock()
				.expect("gui session writer lock should not be poisoned");
			write_session_control_frame(&mut *writer, &GuiSessionClientMessage::Request { id, body })
		};
		if let Err(error) = send_result {
			self.pending
				.lock()
				.expect("gui session pending reply map should not be poisoned")
				.remove(&id);
			return Err(error);
		}
		Ok(())
	}

	fn hydrate_frame(&self, mut frame: GuiRuntimeFrame) -> Result<GuiRuntimeFrame, GlorpError> {
		let Some(document_sync) = frame.document_sync else {
			return Ok(frame);
		};
		let (_, bytes) = self.document_fetch(document_sync.revision)?;
		frame.document_text = Some(
			String::from_utf8(bytes)
				.map_err(|error| GlorpError::transport(format!("document payload is not valid UTF-8: {error}")))?,
		);
		Ok(frame)
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
		match self.request_control(GuiSessionRequest::Call(call))? {
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
	reader: UnixStream, pending: Arc<Mutex<BTreeMap<u64, PendingReply>>>,
	events_tx: mpsc::Sender<GuiSessionHostMessage>,
) {
	thread::spawn(move || {
		let mut reader = reader;
		loop {
			let frame = match read_session_frame(&mut reader) {
				Ok(Some(frame)) => frame,
				Ok(None) => break,
				Err(_) => break,
			};
			match frame {
				GuiSessionFrame::Control(bytes) => {
					let Ok(message) = serde_json::from_slice::<GuiSessionHostMessage>(&bytes) else {
						break;
					};
					match message {
						GuiSessionHostMessage::Reply { id, body } => {
							let mut pending = pending
								.lock()
								.expect("gui session pending reply map should not be poisoned");
							let Some(entry) = pending.get_mut(&id) else {
								continue;
							};
							match entry {
								PendingReply::Control(sender) => {
									let sender = sender.clone();
									pending.remove(&id);
									let _ = sender.send(Ok(body));
								}
								PendingReply::DocumentFetch { sender, response } => match body {
									GuiSessionResponse::DocumentFetch(Ok(meta)) => *response = Some(meta),
									GuiSessionResponse::DocumentFetch(Err(error)) => {
										let sender = sender.clone();
										pending.remove(&id);
										let _ = sender.send(Err(error));
									}
									_ => {
										let sender = sender.clone();
										pending.remove(&id);
										let _ = sender.send(Err(unexpected_response("document fetch reply")));
									}
								},
								PendingReply::SceneFetch { sender, response } => match body {
									GuiSessionResponse::SceneFetch(Ok(GuiSceneFetchResponse::NotModified)) => {
										let sender = sender.clone();
										pending.remove(&id);
										let _ = sender.send(Ok(None));
									}
									GuiSessionResponse::SceneFetch(Ok(GuiSceneFetchResponse::Payload(meta))) => {
										*response = Some(meta);
									}
									GuiSessionResponse::SceneFetch(Err(error)) => {
										let sender = sender.clone();
										pending.remove(&id);
										let _ = sender.send(Err(error));
									}
									_ => {
										let sender = sender.clone();
										pending.remove(&id);
										let _ = sender.send(Err(unexpected_response("scene fetch reply")));
									}
								},
							}
						}
						GuiSessionHostMessage::Changed(delta) => {
							let _ = events_tx.send(GuiSessionHostMessage::Changed(delta));
						}
						GuiSessionHostMessage::Closed => {
							let _ = events_tx.send(GuiSessionHostMessage::Closed);
							break;
						}
						GuiSessionHostMessage::Ready { .. } => {}
					}
				}
				GuiSessionFrame::Payload { header, bytes } => {
					let mut pending = pending
						.lock()
						.expect("gui session pending reply map should not be poisoned");
					let Some(entry) = pending.get_mut(&header.id) else {
						continue;
					};
					match entry {
						PendingReply::DocumentFetch { sender, response } => {
							let Some(meta) = response.take() else {
								let sender = sender.clone();
								pending.remove(&header.id);
								let _ = sender.send(Err(unexpected_response("document fetch payload ordering")));
								continue;
							};
							if header.kind != GuiPayloadKind::DocumentText {
								let sender = sender.clone();
								pending.remove(&header.id);
								let _ = sender.send(Err(unexpected_response("document fetch payload kind")));
								continue;
							}
							let sender = sender.clone();
							pending.remove(&header.id);
							let _ = sender.send(Ok((meta, bytes)));
						}
						PendingReply::SceneFetch { sender, response } => {
							let Some(meta) = response.take() else {
								let sender = sender.clone();
								pending.remove(&header.id);
								let _ = sender.send(Err(unexpected_response("scene fetch payload ordering")));
								continue;
							};
							if header.kind != GuiPayloadKind::Scene {
								let sender = sender.clone();
								pending.remove(&header.id);
								let _ = sender.send(Err(unexpected_response("scene fetch payload kind")));
								continue;
							}
							let sender = sender.clone();
							pending.remove(&header.id);
							let _ = sender.send(Ok(Some((meta, bytes))));
						}
						PendingReply::Control(sender) => {
							let sender = sender.clone();
							pending.remove(&header.id);
							let _ = sender.send(Err(unexpected_response("payload for control request")));
						}
					}
				}
			}
		}

		let waiters = pending
			.lock()
			.expect("gui session pending reply map should not be poisoned")
			.split_off(&0);
		for (_, waiter) in waiters {
			match waiter {
				PendingReply::Control(sender) => {
					let _ = sender.send(Err(GlorpError::transport("gui session reader stopped")));
				}
				PendingReply::DocumentFetch { sender, .. } => {
					let _ = sender.send(Err(GlorpError::transport("gui session reader stopped")));
				}
				PendingReply::SceneFetch { sender, .. } => {
					let _ = sender.send(Err(GlorpError::transport("gui session reader stopped")));
				}
			}
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
