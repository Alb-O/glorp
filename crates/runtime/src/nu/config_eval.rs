use {
	crate::config::ConfigStore,
	glorp_api::{GlorpConfig, GlorpError},
};

pub fn load_config(store: &ConfigStore) -> Result<GlorpConfig, GlorpError> {
	store.load()
}
