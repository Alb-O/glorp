use crate::config::{ConfigStore, ConfigStorePaths};

#[must_use]
pub const fn config_store(paths: ConfigStorePaths) -> ConfigStore {
	ConfigStore::new(paths)
}
