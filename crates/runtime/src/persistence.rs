use {crate::ConfigStore, glorp_api::GlorpError};

struct SurfaceArtifact {
	path: std::path::PathBuf,
	parent_label: &'static str,
	artifact_label: &'static str,
	contents: String,
}

pub fn export_surface_artifacts(store: &ConfigStore) -> Result<(), GlorpError> {
	let _ = sync_surface_artifacts(store)?;
	Ok(())
}

pub fn sync_surface_artifacts(store: &ConfigStore) -> Result<bool, GlorpError> {
	render_surface_artifacts(store)?
		.into_iter()
		.try_fold(false, |changed, artifact| {
			write_artifact_if_changed(&artifact).map(|updated| changed || updated)
		})
}

pub fn ensure_surface_artifacts_current(store: &ConfigStore) -> Result<(), GlorpError> {
	let stale = render_surface_artifacts(store)?
		.into_iter()
		.filter_map(stale_artifact)
		.collect::<Result<Vec<_>, _>>()?;

	if stale.is_empty() {
		return Ok(());
	}

	Err(GlorpError::validation(
		None,
		format!(
			"surface artifacts are stale: {}; run `cargo run -p xtask -- surface`",
			stale.join(", ")
		),
	))
}

fn render_surface_artifacts(store: &ConfigStore) -> Result<Vec<SurfaceArtifact>, GlorpError> {
	let schema = serde_json::to_string_pretty(&glorp_api::glorp_schema())
		.map_err(|error| GlorpError::internal(format!("failed to serialize schema: {error}")))?;
	Ok(vec![
		SurfaceArtifact {
			path: store.paths().schema_path.clone(),
			parent_label: "schema directory",
			artifact_label: "schema",
			contents: schema,
		},
		SurfaceArtifact {
			path: store.paths().nu_module_path.clone(),
			parent_label: "Nu module directory",
			artifact_label: "Nu module",
			contents: glorp_api::render_nu_module(),
		},
		SurfaceArtifact {
			path: store.paths().nu_completions_path.clone(),
			parent_label: "Nu completions directory",
			artifact_label: "Nu completions",
			contents: glorp_api::render_nu_completions(),
		},
	])
}

fn stale_artifact(artifact: SurfaceArtifact) -> Option<Result<String, GlorpError>> {
	match std::fs::read_to_string(&artifact.path) {
		Ok(existing) if existing == artifact.contents => None,
		Ok(_) => Some(Ok(artifact.path.display().to_string())),
		Err(error) if error.kind() == std::io::ErrorKind::NotFound => Some(Ok(artifact.path.display().to_string())),
		Err(error) => Some(Err(GlorpError::transport(format!(
			"failed to read {}: {error}",
			artifact.artifact_label
		)))),
	}
}

fn write_artifact_if_changed(artifact: &SurfaceArtifact) -> Result<bool, GlorpError> {
	let changed = std::fs::read_to_string(&artifact.path).map_or(true, |existing| existing != artifact.contents);
	if changed {
		write_artifact(
			&artifact.path,
			artifact.parent_label,
			artifact.artifact_label,
			&artifact.contents,
		)?;
	}
	Ok(changed)
}

fn write_artifact(
	path: &std::path::Path, parent_label: &str, artifact_label: &str, contents: &str,
) -> Result<(), GlorpError> {
	if let Some(parent) = path.parent() {
		std::fs::create_dir_all(parent)
			.map_err(|error| GlorpError::transport(format!("failed to create {parent_label}: {error}")))?;
	}

	std::fs::write(path, contents)
		.map_err(|error| GlorpError::transport(format!("failed to write {artifact_label}: {error}")))
}
