//! Embedded Nushell-facing helpers owned by the runtime.
//!
//! This module marks a deliberate seam: Nushell is a primary consumer of the
//! public `glorp` surface, but Nushell should not become the owner of canonical
//! runtime state. The runtime embeds just enough Nu-specific logic here to
//! evaluate durable config, export generated surface artifacts, and host script
//! execution against the public contract.
//!
//! # Submodules
//!
//! - [`config_eval`] evaluates the durable Nu config source into typed runtime
//!   config.
//! - [`engine`] builds the constrained Nu engine context needed by runtime
//!   helpers.
//! - [`schema_export`] renders checked-in artifacts derived from the public
//!   schema and call registry.
//! - [`script_host`] exposes a runtime-backed execution environment for
//!   automation flows.
//!
//! # Boundary rule
//!
//! Keep Nu-specific parsing and artifact generation here. If a behavior is
//! public product semantics, model it in `glorp_api`; if it is canonical state
//! mutation, keep it in the main runtime path.

pub mod config_eval;
pub mod engine;
pub mod schema_export;
pub mod script_host;
