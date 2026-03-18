use crate::ConfigStore;

#[must_use]
pub fn config_store(paths: crate::ConfigStorePaths) -> ConfigStore {
	ConfigStore::new(paths)
}
