use {
	clap::{Parser, Subcommand},
	glorp_api::GlorpError,
	glorp_editor::telemetry::init_tracing,
	glorp_runtime::{RuntimeHost, RuntimeOptions, default_runtime_paths},
	glorp_transport::{default_socket_path, ensure_socket_parent, start_server},
	std::{path::PathBuf, process::ExitCode},
	tracing::info,
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
	init_tracing();
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
	}
}

fn serve(paths: glorp_runtime::ConfigStorePaths, socket_path: PathBuf) -> Result<(), GlorpError> {
	info!(
		socket = %socket_path.display(),
		config = %paths.durable_config_path.display(),
		schema = %paths.schema_path.display(),
		"starting host"
	);
	ensure_socket_parent(&socket_path)?;
	let host = RuntimeHost::new(RuntimeOptions { paths })?;
	start_server(socket_path, host)?.wait()
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
}
