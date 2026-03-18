use {crate::ConfigStore, glorp_api::GlorpError};

pub fn export_schema(store: &ConfigStore) -> Result<(), GlorpError> {
	crate::persistence::persist_schema(store)
}
