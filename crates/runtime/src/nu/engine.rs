use crate::ConfigStore;

#[must_use]
pub const fn config_store(paths: crate::ConfigStorePaths) -> ConfigStore {
	ConfigStore::new(paths)
}
