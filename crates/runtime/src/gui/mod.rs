//! Private GUI/runtime session protocol.
//!
//! This is the interactive window's privileged path into the shared runtime. It
//! sits beside the public `glorp_api` surface because it carries GUI-specific
//! conveniences and sync rules.
//!
//! Key behaviors:
//!
//! - edits are revision-based; stale `GuiEditRequest`s are rejected explicitly
//! - edit replies include undo/redo depths so the sidebar can update without an
//!   extra round trip
//! - boot frames and large edit updates may omit inline text payloads once they
//!   exceed `LARGE_PAYLOAD_BYTES`
//! - omitted text is represented by `GuiDocumentSyncRef` and recovered through
//!   `DocumentFetch`
//! - `DocumentFetch` is a resync-to-latest operation, not a historical lookup:
//!   callers ask for at least a revision, and the runtime may answer with a
//!   newer snapshot
//! - future revisions are rejected explicitly instead of silently returning an
//!   unrelated snapshot
//!
//! This protocol is intentionally narrow and specific to the editor window.
//! Other clients should stay on the public call/event surface.

use glorp_api::{GlorpConfig, GlorpOutcome, GlorpRevisions, TextRange};

pub const LARGE_PAYLOAD_BYTES: usize = 4096;

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash)]
pub enum SidebarTab {
	Controls,
	Inspect,
	Perf,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum GuiEditCommand {
	ReplaceRange { range: TextRange, inserted: String },
	Undo,
	Redo,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct GuiEditRequest {
	pub base_revision: u64,
	pub command: GuiEditCommand,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum GuiEditResponse {
	Applied {
		outcome: GlorpOutcome,
		revisions: GlorpRevisions,
		undo_depth: usize,
		redo_depth: usize,
	},
	RejectedStale {
		latest_revision: u64,
		undo_depth: usize,
		redo_depth: usize,
	},
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum GuiDocumentSyncReason {
	Bootstrap,
	LargeEdit,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct GuiDocumentSyncRef {
	pub revision: u64,
	pub bytes: usize,
	pub reason: GuiDocumentSyncReason,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct GuiDocumentFetchRequest {
	/// Minimum editor revision the caller is trying to resynchronize to.
	///
	/// The runtime does not retain historical buffers here; it returns whatever
	/// the current document snapshot is at reply time.
	///
	/// Requests for revisions newer than the current runtime revision are
	/// rejected.
	pub minimum_revision: u64,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct GuiDocumentFetchResponse {
	/// Editor revision of the snapshot carried by the accompanying payload.
	pub revision: u64,
	/// Byte length of the accompanying UTF-8 document payload.
	pub bytes: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct GuiSharedDelta {
	pub outcome: GlorpOutcome,
	pub undo_depth: usize,
	pub redo_depth: usize,
	pub config: Option<GlorpConfig>,
	pub document_sync: Option<GuiDocumentSyncRef>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum GuiSessionRequest {
	Call(glorp_api::GlorpCall),
	Edit(GuiEditRequest),
	GuiFrame,
	DocumentFetch(GuiDocumentFetchRequest),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum GuiSessionResponse {
	Call(Result<glorp_api::GlorpCallResult, glorp_api::GlorpError>),
	Edit(Result<GuiEditResponse, glorp_api::GlorpError>),
	GuiFrame(Result<GuiRuntimeFrame, glorp_api::GlorpError>),
	DocumentFetch(Result<GuiDocumentFetchResponse, glorp_api::GlorpError>),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum GuiSessionClientMessage {
	Request { id: u64, body: GuiSessionRequest },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum GuiSessionHostMessage {
	Ready { frame: Box<GuiRuntimeFrame> },
	Reply { id: u64, body: GuiSessionResponse },
	Changed(GuiSharedDelta),
	Closed,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GuiRuntimeFrame {
	pub config: GlorpConfig,
	pub revisions: GlorpRevisions,
	pub document_text: Option<String>,
	pub undo_depth: usize,
	pub redo_depth: usize,
	pub document_sync: Option<GuiDocumentSyncRef>,
}
