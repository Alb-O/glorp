use {
	crate::commands::all_commands,
	nu_plugin::{Plugin, PluginCommand},
};

pub struct GlorpPlugin;

impl Plugin for GlorpPlugin {
	fn version(&self) -> String {
		env!("CARGO_PKG_VERSION").into()
	}

	fn commands(&self) -> Vec<Box<dyn PluginCommand<Plugin = Self>>> {
		all_commands()
	}
}
