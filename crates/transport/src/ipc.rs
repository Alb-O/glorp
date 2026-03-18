use glorp_api::{
	GlorpError, GlorpEvent, GlorpExec, GlorpOutcome, GlorpQuery, GlorpQueryResult, GlorpStreamToken, GlorpSubscription,
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
