use {crate::ConfigStore, glorp_api::GlorpError};

pub fn persist_schema(store: &ConfigStore) -> Result<(), GlorpError> {
	let schema = serde_json::to_string_pretty(&glorp_api::glorp_schema())
		.map_err(|error| GlorpError::internal(format!("failed to serialize schema: {error}")))?;
	write_artifact(&store.paths().schema_path, "schema directory", "schema", &schema)
}

pub fn export_surface_artifacts(store: &ConfigStore) -> Result<(), GlorpError> {
	persist_schema(store)?;
	write_artifact(
		&store.paths().nu_module_path,
		"Nu module directory",
		"Nu module",
		&glorp_api::render_nu_module(),
	)?;
	write_artifact(
		&store.paths().nu_completions_path,
		"Nu completions directory",
		"Nu completions",
		&glorp_api::render_nu_completions(),
	)
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
