use {
	glorp_api::{GlorpCall, GlorpCallResult, GlorpError},
	glorp_editor::ScenePresentation,
	glorp_runtime::{
		GuiEditRequest, GuiEditResponse, GuiLayoutRequest, GuiRuntimeFrame, GuiSessionClientMessage,
		GuiSessionHostMessage,
	},
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
	GuiFrame(GuiLayoutRequest),
	SceneFetch(GuiLayoutRequest),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum GuiTransportResponse {
	Edit(Box<Result<GuiEditResponse, GlorpError>>),
	GuiFrame(Box<Result<GuiRuntimeFrame, GlorpError>>),
	SceneFetch(Box<Result<ScenePresentation, GlorpError>>),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GuiSessionOpen {
	pub layout: GuiLayoutRequest,
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
