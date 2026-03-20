//! Local transport adapters for the `glorp` runtime.
//!
//! This crate is the boundary between the canonical runtime host and external
//! clients. It carries `glorp_api` types over in-process and Unix-socket
//! boundaries without changing their meaning.
//!
//! Transport layers:
//!
//! - `local` provides an in-process client against a shared host
//! - `client` provides one-shot IPC clients and the long-lived GUI session client
//! - `server` hosts the Unix-socket server and dispatches requests
//! - `ipc` defines the wire framing
//!
//! The public semantic API still lives in `glorp_api`; this crate only moves it.
//! The one private extension is the GUI session protocol, kept in this crate and
//! layered beside the public call vocabulary.
//!
//! In practice there are three paths through this crate:
//!
//! - one-shot public calls over JSON request/response
//! - one-shot GUI helper requests such as `GuiFrame` and `Edit`
//! - a persistent GUI session with streamed `Changed` messages and out-of-band
//!   document payload frames
//!
//! The only transport-routed public semantic today is shared-server shutdown.
//! Everything else either routes to the runtime or stays client-local.
//!
//! The stable repo-local socket contract is exposed by [`default_socket_path`].

mod client;
mod ipc;
mod local;
mod server;

use {
	glorp_api::GlorpError,
	std::{
		path::{Path, PathBuf},
		time::Duration,
	},
};

pub use self::{
	client::{GuiSessionClient, IpcClient, gui_transport_request, socket_is_live, transport_request},
	ipc::{
		GuiSessionOpen, GuiTransportRequest, GuiTransportResponse, ServerRequest, ServerResponse, TransportRequest,
		TransportResponse,
	},
	local::LocalClient,
	server::{IpcServerHandle, start_server, start_server_shared},
};

pub fn default_socket_path(repo_root: impl AsRef<Path>) -> PathBuf {
	repo_root.as_ref().join("glorp.sock")
}

pub fn ensure_socket_parent(socket_path: &Path) -> Result<(), GlorpError> {
	nu_session_ipc::ensure_parent_dir(socket_path).map_err(|error| {
		GlorpError::transport(format!(
			"failed to create socket parent {}: {error}",
			socket_path.parent().unwrap_or_else(|| Path::new(".")).display()
		))
	})
}

pub fn wait_for_socket(socket_path: &Path) -> Result<(), GlorpError> {
	nu_session_ipc::wait_for_live_socket(socket_path, Duration::from_secs(5), client::socket_is_live)
		.map_err(|error| GlorpError::transport(error.to_string()))
}
