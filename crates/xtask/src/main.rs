use {
	clap::{Args, Parser, Subcommand},
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

fn run() -> Result<(), Box<dyn std::error::Error>> {
	match Cli::parse().command {
		Command::Surface(args) => run_surface(args),
	}
}

fn run_surface(args: SurfaceArgs) -> Result<(), Box<dyn std::error::Error>> {
	let repo_root = args.repo_root.unwrap_or_else(xtask::repo_root);
	if args.check {
		xtask::check_surface(&repo_root)?;
		println!("surface is current");
		return Ok(());
	}

	let status = xtask::sync_surface(&repo_root)?;
	if status.changed() {
		println!("surface updated");
	} else {
		println!("surface already up to date");
	}
	Ok(())
}

#[derive(Debug, Parser)]
struct Cli {
	#[command(subcommand)]
	command: Command,
}

#[derive(Debug, Clone, Subcommand)]
enum Command {
	Surface(SurfaceArgs),
}

#[derive(Debug, Clone, Args)]
struct SurfaceArgs {
	#[arg(long)]
	check: bool,
	#[arg(long)]
	repo_root: Option<PathBuf>,
}
