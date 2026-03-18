mod command;
mod config;
mod error;
mod event;
mod helper;
mod query;
mod revision;
mod schema;
mod surface;
mod txn;
mod value;

pub use self::{
	command::*, config::*, error::*, event::*, helper::*, query::*, revision::*, schema::*, surface::*, txn::*,
	value::*,
};

pub type GlorpStreamToken = u64;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum GlorpSubscription {
	Changes,
}

pub trait GlorpHost {
	fn execute(&mut self, exec: GlorpExec) -> Result<GlorpOutcome, GlorpError>;
	fn query(&mut self, query: GlorpQuery) -> Result<GlorpQueryResult, GlorpError>;
	fn subscribe(&mut self, request: GlorpSubscription) -> Result<GlorpStreamToken, GlorpError>;
	fn next_event(&mut self, token: GlorpStreamToken) -> Result<Option<GlorpEvent>, GlorpError>;
	fn unsubscribe(&mut self, token: GlorpStreamToken) -> Result<(), GlorpError>;
}
