mod client;
mod ipc;
mod local;
mod server;

use std::path::{Path, PathBuf};

pub use self::{
	client::{IpcClient, gui_transport_request, socket_is_live, transport_request},
	ipc::{
		GuiTransportRequest, GuiTransportResponse, ServerRequest, ServerResponse, TransportRequest, TransportResponse,
	},
	local::LocalClient,
	server::{IpcServerHandle, start_server, start_server_shared},
};

pub fn default_socket_path(repo_root: impl AsRef<Path>) -> PathBuf {
	repo_root.as_ref().join("glorp.sock")
}
