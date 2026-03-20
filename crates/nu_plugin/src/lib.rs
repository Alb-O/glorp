//! Nushell plugin adapter for the public `glorp` surface.
//!
//! This crate is intentionally thin. It turns the public semantic API into
//! Nushell commands over transport rather than adding a parallel business-logic
//! layer.
//!
//! Boundaries are simple:
//!
//! - command vocabulary comes from `glorp_api`
//! - transport attachment comes from `glorp_transport`
//! - shell-specific registration, completion, and argument bridging live here
//! - the current client-routed calls also live here, notably `session-attach`
//!   discovery and `config-validate`
//!
//! If behavior feels product-level rather than shell-level, it probably belongs
//! in `glorp_api` or `glorp_runtime`.

mod commands;
mod completions;
mod plugin;

pub use self::plugin::GlorpPlugin;
