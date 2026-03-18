use {
	crate::{TransportRequest, TransportResponse},
	glorp_api::{
		GlorpCommand, GlorpError, GlorpEvent, GlorpHost, GlorpOutcome, GlorpQuery, GlorpQueryResult, GlorpStreamToken,
		GlorpSubscription,
	},
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
	if !socket_path.exists() {
		return false;
	}

	let mut client = IpcClient::new(socket_path);
	matches!(
		client.query(GlorpQuery::Capabilities),
		Ok(GlorpQueryResult::Capabilities(_))
	)
}

pub fn transport_request<T>(socket_path: &Path, request: &TransportRequest) -> Result<T, GlorpError>
where
	T: for<'de> serde::Deserialize<'de>, {
	let mut stream = UnixStream::connect(socket_path)
		.map_err(|error| GlorpError::transport(format!("failed to connect to {}: {error}", socket_path.display())))?;
	let payload = serde_json::to_string(request)
		.map_err(|error| GlorpError::internal(format!("failed to encode request: {error}")))?;
	writeln!(stream, "{payload}")
		.map_err(|error| GlorpError::transport(format!("failed to write request: {error}")))?;

	let mut response = String::new();
	BufReader::new(stream)
		.read_line(&mut response)
		.map_err(|error| GlorpError::transport(format!("failed to read response: {error}")))?;

	serde_json::from_str(&response).map_err(|error| GlorpError::internal(format!("failed to decode response: {error}")))
}

impl GlorpHost for IpcClient {
	fn execute(&mut self, command: GlorpCommand) -> Result<GlorpOutcome, GlorpError> {
		match self.response(&TransportRequest::Execute(command))? {
			TransportResponse::Execute(result) => result,
			_ => Err(GlorpError::transport("unexpected execute response")),
		}
	}

	fn query(&mut self, query: GlorpQuery) -> Result<GlorpQueryResult, GlorpError> {
		match self.response(&TransportRequest::Query(query))? {
			TransportResponse::Query(result) => *result,
			_ => Err(GlorpError::transport("unexpected query response")),
		}
	}

	fn subscribe(&mut self, request: GlorpSubscription) -> Result<GlorpStreamToken, GlorpError> {
		match self.response(&TransportRequest::Subscribe(request))? {
			TransportResponse::Subscribe(result) => result,
			_ => Err(GlorpError::transport("unexpected subscribe response")),
		}
	}

	fn next_event(&mut self, token: GlorpStreamToken) -> Result<Option<GlorpEvent>, GlorpError> {
		match self.response(&TransportRequest::NextEvent(token))? {
			TransportResponse::NextEvent(result) => result,
			_ => Err(GlorpError::transport("unexpected next-event response")),
		}
	}

	fn unsubscribe(&mut self, token: GlorpStreamToken) -> Result<(), GlorpError> {
		match self.response(&TransportRequest::Unsubscribe(token))? {
			TransportResponse::Unsubscribe(result) => result,
			_ => Err(GlorpError::transport("unexpected unsubscribe response")),
		}
	}
}

impl IpcClient {
	fn response(&self, request: &TransportRequest) -> Result<TransportResponse, GlorpError> {
		transport_request(&self.socket_path, request)
	}
}
