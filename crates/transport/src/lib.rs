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
	client::{IpcClient, gui_transport_request, socket_is_live, transport_request},
	ipc::{
		GuiTransportRequest, GuiTransportResponse, ServerRequest, ServerResponse, TransportRequest, TransportResponse,
	},
	local::LocalClient,
	server::{IpcServerHandle, start_server, start_server_shared},
};

pub fn default_socket_path(repo_root: impl AsRef<Path>) -> PathBuf {
	nu_session_protocol_glorp::default_socket_path(repo_root)
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
