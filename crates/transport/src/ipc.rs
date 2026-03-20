//! IPC framing for one-shot requests and persistent GUI sessions.
//!
//! The crate uses two wire shapes on purpose:
//!
//! - normal public/GUI requests are newline-delimited JSON
//! - persistent GUI sessions use a compact framed protocol
//!
//! Session frames come in two kinds:
//!
//! - kind `1`: JSON control messages
//! - kind `2`: binary payload messages with a JSON header and raw bytes
//!
//! That split lets the GUI move large document text out of JSON when an inline
//! `GuiRuntimeFrame` or `GuiSharedDelta` would be too large.

use {
	glorp_api::{GlorpCall, GlorpCallResult, GlorpError},
	glorp_runtime::{
		GuiDocumentFetchRequest, GuiEditRequest, GuiEditResponse, GuiRuntimeFrame, GuiSessionClientMessage,
		GuiSessionHostMessage,
	},
	serde::{Serialize, de::DeserializeOwned},
	std::io::{Read, Write},
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum TransportRequest {
	Call(GlorpCall),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum TransportResponse {
	Call(Box<Result<GlorpCallResult, GlorpError>>),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum GuiTransportRequest {
	Edit(GuiEditRequest),
	GuiFrame,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum GuiTransportResponse {
	Edit(Box<Result<GuiEditResponse, GlorpError>>),
	GuiFrame(Box<Result<GuiRuntimeFrame, GlorpError>>),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GuiSessionOpen {}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum GuiPayloadKind {
	DocumentText,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ServerRequest {
	Public(TransportRequest),
	Gui(GuiTransportRequest),
	GuiSessionOpen(GuiSessionOpen),
	GuiSessionMessage(GuiSessionClientMessage),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ServerResponse {
	Public(TransportResponse),
	Gui(GuiTransportResponse),
	GuiSessionReady(GuiSessionHostMessage),
	GuiSessionMessage(GuiSessionHostMessage),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GuiSessionPayloadHeader {
	pub id: u64,
	pub kind: GuiPayloadKind,
}

#[derive(Debug)]
pub enum GuiSessionFrame {
	Control(Vec<u8>),
	Payload {
		header: GuiSessionPayloadHeader,
		bytes: Vec<u8>,
	},
}

pub fn write_session_control_frame<Message>(stream: &mut impl Write, message: &Message) -> Result<(), GlorpError>
where
	Message: Serialize, {
	let payload = serde_json::to_vec(message)
		.map_err(|error| GlorpError::internal(format!("failed to encode session control frame: {error}")))?;
	write_session_frame(stream, 1, &payload)
}

pub fn write_session_payload_frame(
	stream: &mut impl Write, header: &GuiSessionPayloadHeader, bytes: &[u8],
) -> Result<(), GlorpError> {
	let mut payload = serde_json::to_vec(header)
		.map_err(|error| GlorpError::internal(format!("failed to encode session payload header: {error}")))?;
	payload.push(b'\n');
	payload.extend_from_slice(bytes);
	write_session_frame(stream, 2, &payload)
}

pub fn read_session_control_frame<Message>(reader: &mut impl Read) -> Result<Option<Message>, GlorpError>
where
	Message: DeserializeOwned, {
	match read_session_frame(reader)? {
		Some(GuiSessionFrame::Control(bytes)) => serde_json::from_slice(&bytes)
			.map(Some)
			.map_err(|error| GlorpError::internal(format!("failed to decode session control frame: {error}"))),
		Some(GuiSessionFrame::Payload { .. }) => Err(GlorpError::transport("unexpected session payload frame")),
		None => Ok(None),
	}
}

pub fn read_session_frame(reader: &mut impl Read) -> Result<Option<GuiSessionFrame>, GlorpError> {
	let Some(kind) = read_optional_byte(reader)? else {
		return Ok(None);
	};
	let length = read_u64(reader)? as usize;
	let mut payload = vec![0; length];
	reader
		.read_exact(&mut payload)
		.map_err(|error| GlorpError::transport(format!("failed to read session frame payload: {error}")))?;
	match kind {
		1 => Ok(Some(GuiSessionFrame::Control(payload))),
		2 => {
			let split = payload
				.iter()
				.position(|&byte| byte == b'\n')
				.ok_or_else(|| GlorpError::internal("session payload frame missing header separator"))?;
			let header = serde_json::from_slice::<GuiSessionPayloadHeader>(&payload[..split])
				.map_err(|error| GlorpError::internal(format!("failed to decode session payload header: {error}")))?;
			Ok(Some(GuiSessionFrame::Payload {
				header,
				bytes: payload[split + 1..].to_vec(),
			}))
		}
		other => Err(GlorpError::transport(format!("unknown session frame kind `{other}`"))),
	}
}

pub fn gui_document_request(revision: u64) -> glorp_runtime::GuiSessionRequest {
	glorp_runtime::GuiSessionRequest::DocumentFetch(GuiDocumentFetchRequest { revision })
}

fn write_session_frame(stream: &mut impl Write, kind: u8, payload: &[u8]) -> Result<(), GlorpError> {
	stream
		.write_all(&[kind])
		.map_err(|error| GlorpError::transport(format!("failed to write session frame kind: {error}")))?;
	stream
		.write_all(&(payload.len() as u64).to_be_bytes())
		.map_err(|error| GlorpError::transport(format!("failed to write session frame length: {error}")))?;
	stream
		.write_all(payload)
		.map_err(|error| GlorpError::transport(format!("failed to write session frame payload: {error}")))
}

fn read_optional_byte(reader: &mut impl Read) -> Result<Option<u8>, GlorpError> {
	let mut byte = [0_u8; 1];
	match reader.read(&mut byte) {
		Ok(0) => Ok(None),
		Ok(1) => Ok(Some(byte[0])),
		Ok(_) => unreachable!(),
		Err(error) => Err(GlorpError::transport(format!(
			"failed to read session frame kind: {error}"
		))),
	}
}

fn read_u64(reader: &mut impl Read) -> Result<u64, GlorpError> {
	let mut bytes = [0_u8; 8];
	reader
		.read_exact(&mut bytes)
		.map_err(|error| GlorpError::transport(format!("failed to read session frame length: {error}")))?;
	Ok(u64::from_be_bytes(bytes))
}
