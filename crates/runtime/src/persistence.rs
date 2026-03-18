use {crate::ConfigStore, glorp_api::GlorpError};

pub fn persist_schema(store: &ConfigStore) -> Result<(), GlorpError> {
	if let Some(parent) = store.paths().schema_path.parent() {
		std::fs::create_dir_all(parent)
			.map_err(|error| GlorpError::transport(format!("failed to create schema directory: {error}")))?;
	}

	let schema = serde_json::to_string_pretty(&glorp_api::glorp_schema())
		.map_err(|error| GlorpError::internal(format!("failed to serialize schema: {error}")))?;

	std::fs::write(&store.paths().schema_path, schema)
		.map_err(|error| GlorpError::transport(format!("failed to write schema: {error}")))
}
