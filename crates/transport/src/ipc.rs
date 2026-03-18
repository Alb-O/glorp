use {
	glorp_api::{GlorpCall, GlorpCallResult, GlorpError},
	glorp_runtime::{GuiCommand, GuiTransportFrame},
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
	ExecuteGui(GuiCommand),
	GuiFrame,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum GuiTransportResponse {
	ExecuteGui(Result<(), GlorpError>),
	GuiFrame(Box<Result<GuiTransportFrame, GlorpError>>),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ServerRequest {
	Public(TransportRequest),
	Gui(GuiTransportRequest),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ServerResponse {
	Public(TransportResponse),
	Gui(GuiTransportResponse),
}
