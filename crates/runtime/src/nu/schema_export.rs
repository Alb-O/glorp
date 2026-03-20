use {
	crate::config::{ConfigStore, export_surface_artifacts},
	glorp_api::GlorpError,
};

pub fn export_schema(store: &ConfigStore) -> Result<(), GlorpError> {
	export_surface_artifacts(store)
}
