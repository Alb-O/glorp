use glorp_api::{
	GlorpCommand, GlorpError, GlorpEvent, GlorpOutcome, GlorpQuery, GlorpQueryResult, GlorpStreamToken,
	GlorpSubscription,
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum TransportRequest {
	Execute(GlorpCommand),
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
