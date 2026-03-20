//! Canonical runtime host for `glorp`.
//!
//! This crate is the state-owning middle layer. It sits between `glorp_api` and
//! the adapters that use it: transport, GUI, and Nushell integration.
//!
//! It loads and persists durable config, owns canonical document/config
//! revisions, executes public calls and transactions, publishes public and
//! GUI-private change streams, and exposes a few host conveniences for GUI boot
//! and large-document sync.
//!
//! The current runtime is intentionally document-centric. It owns durable text,
//! undo/redo history, config, and revision counters. The richer interactive
//! `EditorEngine` and scene inspection stack live in the GUI and stay in sync
//! through runtime outcomes.
//!
//! Important seams:
//!
//! - `runtime` holds the main host object and lifecycle
//! - `execute` maps public calls to runtime behavior
//! - `state` owns checkpointable canonical state
//! - `gui` defines the private GUI session protocol that sits beside the public API
//! - [`nu`] holds embedded Nushell helpers for config evaluation and surface export
//!
//! Rule of thumb: if it must survive transport/process boundaries, model it in
//! `glorp_api`; if it only exists to make the editor window work, keep it in the
//! private GUI/session path.

mod config_store;
mod events;
mod execute;
mod gui;
mod host;
pub mod nu;
mod persistence;
mod project;
mod runtime;
mod state;

pub use self::{
	config_store::{ConfigStore, ConfigStorePaths},
	gui::{
		GuiDocumentFetchRequest, GuiDocumentFetchResponse, GuiDocumentSyncReason, GuiDocumentSyncRef, GuiEditCommand,
		GuiEditRequest, GuiEditResponse, GuiRuntimeFrame, GuiSessionClientMessage, GuiSessionHostMessage,
		GuiSessionRequest, GuiSessionResponse, GuiSharedDelta, LARGE_PAYLOAD_BYTES, SidebarTab,
	},
	host::RuntimeHost,
	persistence::{ensure_surface_artifacts_current, export_surface_artifacts, sync_surface_artifacts},
	runtime::{RuntimeOptions, default_runtime_paths},
	state::DEFAULT_LAYOUT_WIDTH,
};
