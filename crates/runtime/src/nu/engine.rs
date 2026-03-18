use crate::ConfigStore;

pub fn config_store(paths: crate::ConfigStorePaths) -> ConfigStore {
	ConfigStore::new(paths)
}
