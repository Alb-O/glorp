//! Public semantic contract for `glorp`.
//!
//! This crate defines the durable product surface. Anything that crosses
//! process, scripting, or client boundaries should use these types instead of
//! reaching into runtime internals.
//!
//! The split is:
//!
//! - `glorp_api` defines the call, result, event, config, revision, and schema vocabulary.
//! - `glorp_runtime` executes that contract against canonical text/config/history state.
//! - `glorp_transport`, `glorp_gui`, and `glorp_nu_plugin` adapt to it without redefining it.
//!
//! The public surface is intentionally narrower than the full GUI/editor stack.
//! It covers durable facts such as document text, document summary state,
//! config, capabilities, and revisioned outcomes. Layout geometry, inspect
//! state, and GUI session plumbing stay out of it.
//!
//! It owns raw envelopes like [`GlorpCall`] and [`GlorpCallResult`], typed
//! descriptors in `surface`, durable config/outcome types, and reflection data
//! used by generated artifacts and Nushell tooling.
//!
//! It does not own persistence, client-side layout, IPC framing, or GUI-only
//! hydration details.
//!
//! Read `surface` first for the generated call registry, then `schema`,
//! `config`, `command`, `query`, and `event` for the public payload types.

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
