mod catalog;
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

pub trait GlorpCaller {
	fn call(&mut self, call: GlorpCall) -> Result<GlorpCallResult, GlorpError>;
}
