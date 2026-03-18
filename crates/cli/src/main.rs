mod commands;
mod output;

use {clap::Parser, std::process::ExitCode};

fn main() -> ExitCode {
	match commands::Cli::parse().run() {
		Ok(()) => ExitCode::SUCCESS,
		Err(error) => {
			eprintln!("{error}");
			ExitCode::FAILURE
		}
	}
}
