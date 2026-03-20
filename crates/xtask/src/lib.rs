//! Workspace maintenance tasks for checked-in surface artifacts.
//!
//! `xtask surface` is the operational seam that keeps generated Rust helpers,
//! checked-in schema artifacts, and generated Nushell assets aligned. The rest
//! of the workspace assumes those artifacts are current.

use {
	glorp_api_codegen::{SURFACE_COMMAND, generated_calls_are_current, generated_calls_path, sync_generated_calls},
	glorp_runtime::{ConfigStore, default_runtime_paths, ensure_surface_artifacts_current, sync_surface_artifacts},
	std::{
		error::Error,
		io,
		path::{Path, PathBuf},
	},
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SurfaceStatus {
	pub generated_calls_changed: bool,
	pub artifacts_changed: bool,
}

impl SurfaceStatus {
	#[must_use]
	pub const fn changed(self) -> bool {
		self.generated_calls_changed || self.artifacts_changed
	}
}

pub fn repo_root() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR"))
		.join("../..")
		.components()
		.as_path()
		.to_path_buf()
}

pub fn sync_surface(repo_root: &Path) -> Result<SurfaceStatus, Box<dyn Error>> {
	let store = ConfigStore::new(default_runtime_paths(repo_root));
	Ok(SurfaceStatus {
		generated_calls_changed: sync_generated_calls()?,
		artifacts_changed: sync_surface_artifacts(&store)?,
	})
}

pub fn check_surface(repo_root: &Path) -> Result<(), Box<dyn Error>> {
	let mut stale = Vec::new();
	if !generated_calls_are_current()? {
		stale.push(generated_calls_path().display().to_string());
	}

	let store = ConfigStore::new(default_runtime_paths(repo_root));
	if let Err(error) = ensure_surface_artifacts_current(&store) {
		stale.push(error.to_string());
	}

	if stale.is_empty() {
		return Ok(());
	}

	Err(io::Error::other(format!(
		"surface is stale: {}; run `{SURFACE_COMMAND}`",
		stale.join("; ")
	))
	.into())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn checked_in_surface_is_current() {
		check_surface(&repo_root()).expect("checked-in generated surface should be current");
	}
}
