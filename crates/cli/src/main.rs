use {
	clap::{Parser, Subcommand},
	glorp_api::GlorpError,
	glorp_runtime::{ConfigStore, RuntimeHost, RuntimeOptions, default_runtime_paths, export_surface_artifacts},
	glorp_transport::{default_socket_path, start_server},
	std::{path::PathBuf, process::ExitCode},
};

fn main() -> ExitCode {
	match run() {
		Ok(()) => ExitCode::SUCCESS,
		Err(error) => {
			eprintln!("{error}");
			ExitCode::FAILURE
		}
	}
}

fn run() -> Result<(), GlorpError> {
	let cli = Cli::parse();
	let repo_root = cli.repo_root.map_or_else(
		|| {
			std::env::current_dir()
				.map_err(|error| GlorpError::transport(format!("failed to determine current directory: {error}")))
		},
		Ok,
	)?;
	let socket_path = cli.socket.unwrap_or_else(|| default_socket_path(&repo_root));
	let paths = default_runtime_paths(&repo_root);

	match cli.command.unwrap_or(Command::Serve) {
		Command::Serve => serve(paths, socket_path),
		Command::ExportSurface => export(paths),
	}
}

fn serve(paths: glorp_runtime::ConfigStorePaths, socket_path: PathBuf) -> Result<(), GlorpError> {
	ensure_parent(&socket_path, "socket parent")?;
	let host = RuntimeHost::new(RuntimeOptions { paths })?;
	start_server(socket_path, host)?.wait()
}

fn export(paths: glorp_runtime::ConfigStorePaths) -> Result<(), GlorpError> {
	export_surface_artifacts(&ConfigStore::new(paths))
}

fn ensure_parent(path: &std::path::Path, label: &str) -> Result<(), GlorpError> {
	path.parent().map_or(Ok(()), |parent| {
		std::fs::create_dir_all(parent)
			.map_err(|error| GlorpError::transport(format!("failed to create {label} {}: {error}", parent.display())))
	})
}

#[derive(Debug, Parser)]
struct Cli {
	#[arg(long)]
	socket: Option<PathBuf>,
	#[arg(long)]
	repo_root: Option<PathBuf>,
	#[command(subcommand)]
	command: Option<Command>,
}

#[derive(Debug, Clone, Copy, Subcommand)]
enum Command {
	Serve,
	ExportSurface,
}
