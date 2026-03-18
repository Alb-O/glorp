use {
	glorp_api::{GlorpConfig, GlorpError, GlorpValue},
	std::{path::PathBuf, process::Command},
};

#[derive(Debug, Clone)]
pub struct ConfigStorePaths {
	pub durable_config_path: PathBuf,
	pub schema_path: PathBuf,
	pub nu_module_path: PathBuf,
	pub nu_completions_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ConfigStore {
	paths: ConfigStorePaths,
}

impl ConfigStore {
	#[must_use]
	pub const fn new(paths: ConfigStorePaths) -> Self {
		Self { paths }
	}

	#[must_use]
	pub const fn paths(&self) -> &ConfigStorePaths {
		&self.paths
	}

	pub fn load(&self) -> Result<GlorpConfig, GlorpError> {
		if !self.paths.durable_config_path.exists() {
			self.save(&GlorpConfig::default())?;
		}

		let output = Command::new("nu")
			.args([
				"-c",
				&format!(
					"use {} *; $config | to json -r",
					self.paths.durable_config_path.display()
				),
			])
			.output()
			.map_err(|error| GlorpError::transport(format!("failed to execute nu: {error}")))?;

		if !output.status.success() {
			return Err(GlorpError::validation(
				None,
				format!(
					"failed to evaluate config.nu: {}",
					String::from_utf8_lossy(&output.stderr).trim()
				),
			));
		}

		let value: serde_json::Value = serde_json::from_slice(&output.stdout)
			.map_err(|error| GlorpError::validation(None, format!("invalid JSON from nu: {error}")))?;

		config_from_json(value)
	}

	pub fn save(&self, config: &GlorpConfig) -> Result<(), GlorpError> {
		if let Some(parent) = self.paths.durable_config_path.parent() {
			std::fs::create_dir_all(parent)
				.map_err(|error| GlorpError::transport(format!("failed to create config directory: {error}")))?;
		}

		std::fs::write(&self.paths.durable_config_path, render_config(config))
			.map_err(|error| GlorpError::transport(format!("failed to write config: {error}")))
	}
}

fn config_from_json(value: serde_json::Value) -> Result<GlorpConfig, GlorpError> {
	let mut config = GlorpConfig::default();
	let GlorpValue::Record(root) = GlorpValue::from(value) else {
		return Err(GlorpError::validation(None, "config.nu must evaluate to a record"));
	};

	root.into_iter().try_for_each(|(namespace, value)| match value {
		GlorpValue::Record(fields) => fields
			.into_iter()
			.try_for_each(|(field, value)| config.set_path(&format!("{namespace}.{field}"), &value)),
		other => Err(GlorpError::validation(
			None,
			format!("config namespace `{namespace}` must be a record, got {}", other.kind()),
		)),
	})?;

	Ok(config)
}

#[must_use]
pub fn render_config(config: &GlorpConfig) -> String {
	format!(
		"export const config = {{\n  editor: {{\n    preset: {}\n    font: \"{}\"\n    shaping: \"{}\"\n    wrapping: \"{}\"\n    font_size: {}\n    line_height: {}\n  }}\n}}\n",
		render_optional_preset(config.editor.preset),
		<glorp_api::FontChoice as glorp_api::EnumValue>::as_ref(config.editor.font),
		<glorp_api::ShapingChoice as glorp_api::EnumValue>::as_ref(config.editor.shaping),
		<glorp_api::WrapChoice as glorp_api::EnumValue>::as_ref(config.editor.wrapping),
		config.editor.font_size,
		config.editor.line_height,
	)
}

fn render_optional_preset(preset: Option<glorp_api::SamplePreset>) -> String {
	preset.map_or_else(
		|| "null".into(),
		|preset| {
			format!(
				"\"{}\"",
				<glorp_api::SamplePreset as glorp_api::EnumValue>::as_ref(preset)
			)
		},
	)
}
