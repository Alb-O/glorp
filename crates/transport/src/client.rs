use {
	crate::{
		GuiTransportRequest, GuiTransportResponse, ServerRequest, ServerResponse, TransportRequest, TransportResponse,
	},
	glorp_api::{GlorpCall, GlorpCallDescriptor, GlorpCallResult, GlorpCaller, GlorpError},
	serde::{Serialize, de::DeserializeOwned},
	std::{
		io::{BufRead, BufReader, Write},
		os::unix::net::UnixStream,
		path::{Path, PathBuf},
	},
};

#[derive(Debug, Clone)]
pub struct IpcClient {
	socket_path: PathBuf,
}

impl IpcClient {
	pub fn new(socket_path: impl Into<PathBuf>) -> Self {
		Self {
			socket_path: socket_path.into(),
		}
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
			ServerResponse::Gui(_) => None,
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
			ServerResponse::Public(_) => None,
		},
	)
}

fn request_response<Req, Resp>(socket_path: &Path, request: Req) -> Result<Resp, GlorpError>
where
	Req: Serialize,
	Resp: DeserializeOwned, {
	let mut stream = UnixStream::connect(socket_path)
		.map_err(|error| GlorpError::transport(format!("failed to connect to {}: {error}", socket_path.display())))?;
	let payload = serde_json::to_string(&request)
		.map_err(|error| GlorpError::internal(format!("failed to encode request: {error}")))?;
	writeln!(stream, "{payload}")
		.map_err(|error| GlorpError::transport(format!("failed to write request: {error}")))?;

	let mut response = String::new();
	BufReader::new(stream)
		.read_line(&mut response)
		.map_err(|error| GlorpError::transport(format!("failed to read response: {error}")))?;

	serde_json::from_str(&response).map_err(|error| GlorpError::internal(format!("failed to decode response: {error}")))
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

impl IpcClient {
	fn response(&self, request: TransportRequest) -> Result<TransportResponse, GlorpError> {
		transport_request(&self.socket_path, request)
	}
}

fn expect_response<T, Response>(
	response: Result<Response, GlorpError>, kind: &str, extract: impl FnOnce(Response) -> Option<T>,
) -> Result<T, GlorpError> {
	extract(response?).ok_or_else(|| unexpected_response(kind))
}

fn unexpected_response(kind: &str) -> GlorpError {
	GlorpError::transport(format!("unexpected {kind} response"))
}
