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
//! - `host` holds the runtime object, execution path, checkpoints, and change streams
//! - `config` owns durable config load/save and checked-in surface artifacts
//! - `gui` defines the private GUI session protocol that sits beside the public API
//! - [`nu`] holds embedded Nushell helpers for config evaluation and surface export
//!
//! Rule of thumb: if it must survive transport/process boundaries, model it in
//! `glorp_api`; if it only exists to make the editor window work, keep it in the
//! private GUI/session path.

mod config;
mod gui;
mod host;
pub mod nu;

pub use self::{
	config::{
		ConfigStore, ConfigStorePaths, ensure_surface_artifacts_current, export_surface_artifacts,
		sync_surface_artifacts,
	},
	gui::{
		GuiDocumentFetchRequest, GuiDocumentFetchResponse, GuiDocumentSyncReason, GuiDocumentSyncRef, GuiEditCommand,
		GuiEditRequest, GuiEditResponse, GuiRuntimeFrame, GuiSessionClientMessage, GuiSessionHostMessage,
		GuiSessionRequest, GuiSessionResponse, GuiSharedDelta, LARGE_PAYLOAD_BYTES, SidebarTab,
	},
	host::{DEFAULT_LAYOUT_WIDTH, RuntimeHost, RuntimeOptions, default_runtime_paths},
};
