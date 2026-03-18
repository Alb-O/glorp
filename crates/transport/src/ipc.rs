use {
	glorp_api::{
		GlorpError, GlorpEvent, GlorpExec, GlorpOutcome, GlorpQuery, GlorpQueryResult, GlorpStreamToken,
		GlorpSubscription,
	},
	glorp_runtime::{GuiCommand, GuiTransportFrame},
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum TransportRequest {
	Execute(GlorpExec),
	Query(GlorpQuery),
	Subscribe(GlorpSubscription),
	NextEvent(GlorpStreamToken),
	Unsubscribe(GlorpStreamToken),
	Shutdown,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum TransportResponse {
	Execute(Result<GlorpOutcome, GlorpError>),
	Query(Box<Result<GlorpQueryResult, GlorpError>>),
	Subscribe(Result<GlorpStreamToken, GlorpError>),
	NextEvent(Result<Option<GlorpEvent>, GlorpError>),
	Unsubscribe(Result<(), GlorpError>),
	Shutdown(Result<(), GlorpError>),
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
