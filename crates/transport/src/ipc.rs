use {
	glorp_api::{GlorpCall, GlorpCallResult, GlorpError},
	glorp_runtime::{GuiCommand, GuiEditRequest, GuiEditResponse, GuiLayoutRequest, GuiRuntimeFrame},
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
	ExecuteGui {
		layout: GuiLayoutRequest,
		command: GuiCommand,
	},
	Edit(GuiEditRequest),
	GuiFrame(GuiLayoutRequest),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum GuiTransportResponse {
	ExecuteGui(Result<(), GlorpError>),
	Edit(Box<Result<GuiEditResponse, GlorpError>>),
	GuiFrame(Box<Result<GuiRuntimeFrame, GlorpError>>),
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
