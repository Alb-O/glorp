mod canvas_view;
mod editor;
mod overlay;
mod overlay_view;
mod perf;
mod presentation;
mod runtime_shell;
mod scene;
mod scene_view;
mod telemetry;
mod text_view;
mod types;
mod ui;

use {
	clap::Parser,
	glorp_api::GlorpError,
	glorp_gui::GuiLaunchOptions,
	runtime_shell::RuntimeShell,
	std::{path::PathBuf, process::ExitCode, sync::Mutex},
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
	telemetry::init_tracing();
	let cli = Cli::parse();
	let repo_root = cli.repo_root.map_or_else(
		|| {
			std::env::current_dir()
				.map_err(|error| GlorpError::transport(format!("failed to determine current directory: {error}")))
		},
		Ok,
	)?;
	let mut launch = GuiLaunchOptions::for_repo_root(repo_root);
	if let Some(socket_path) = cli.socket {
		launch.socket_path = socket_path;
	}

	let boot = Mutex::new(Some(launch));

	iced::application(
		move || RuntimeShell::boot(boot.lock().expect("boot mutex").take().expect("booted once")),
		RuntimeShell::update,
		RuntimeShell::view,
	)
	.subscription(RuntimeShell::subscription)
	.theme(RuntimeShell::theme)
	.title(RuntimeShell::title)
	.window_size([1400.0, 920.0])
	.centered()
	.run()
	.map_err(|error| GlorpError::internal(format!("GUI application failed: {error}")))
}

#[derive(Debug, Parser, Clone)]
struct Cli {
	#[arg(long)]
	socket: Option<PathBuf>,
	#[arg(long)]
	repo_root: Option<PathBuf>,
}
