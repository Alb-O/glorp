mod client;
mod ipc;
mod local;
mod server;

use std::path::PathBuf;

pub use self::{
	client::{IpcClient, socket_is_live, transport_request},
	ipc::{TransportRequest, TransportResponse},
	local::LocalClient,
	server::{IpcServerHandle, start_server},
};

pub fn default_socket_path(repo_root: impl Into<PathBuf>) -> PathBuf {
	repo_root.into().join("glorp.sock")
}
