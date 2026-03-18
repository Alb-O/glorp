use crate::{ConfigAssignment, ConfigPath, GlorpError, GlorpValue, schema::glorp_schema};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct GlorpConfig {
	pub editor: EditorConfig,
	pub inspect: InspectConfig,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct EditorConfig {
	pub preset: Option<crate::SamplePreset>,
	pub font: crate::FontChoice,
	pub shaping: crate::ShapingChoice,
	pub wrapping: crate::WrapChoice,
	pub font_size: f32,
	pub line_height: f32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct InspectConfig {
	pub show_baselines: bool,
	pub show_hitboxes: bool,
}

impl Default for GlorpConfig {
	fn default() -> Self {
		Self {
			editor: EditorConfig {
				preset: Some(crate::SamplePreset::Tall),
				font: crate::FontChoice::JetBrainsMono,
				shaping: crate::ShapingChoice::Advanced,
				wrapping: crate::WrapChoice::Word,
				font_size: 24.0,
				line_height: 32.0,
			},
			inspect: InspectConfig {
				show_baselines: false,
				show_hitboxes: false,
			},
		}
	}
}

impl GlorpConfig {
	pub fn patch(&mut self, values: &[ConfigAssignment]) -> Result<Vec<ConfigPath>, GlorpError> {
		values
			.iter()
			.map(|assignment| {
				self.set_path(&assignment.path, assignment.value.clone())?;
				Ok(assignment.path.clone())
			})
			.collect()
	}

	pub fn set_path(&mut self, path: &str, value: GlorpValue) -> Result<(), GlorpError> {
		match path {
			"editor.preset" => {
				self.editor.preset = match value {
					GlorpValue::Null => None,
					GlorpValue::String(value) => Some(parse_enum::<crate::SamplePreset>(path, &value)?),
					other => return Err(type_error(path, "string or null", other.kind())),
				};
			}
			"editor.font" => self.editor.font = parse_string_enum(path, value)?,
			"editor.shaping" => self.editor.shaping = parse_string_enum(path, value)?,
			"editor.wrapping" => self.editor.wrapping = parse_string_enum(path, value)?,
			"editor.font_size" => self.editor.font_size = parse_f32(path, &value)?,
			"editor.line_height" => self.editor.line_height = parse_f32(path, &value)?,
			"inspect.show_baselines" => self.inspect.show_baselines = parse_bool(path, &value)?,
			"inspect.show_hitboxes" => self.inspect.show_hitboxes = parse_bool(path, &value)?,
			_ => return Err(unknown_path(path)),
		}

		Ok(())
	}

	pub fn reset_path(&mut self, path: &str) -> Result<(), GlorpError> {
		let default = Self::default();

		match path {
			"editor.preset" => self.editor.preset = default.editor.preset,
			"editor.font" => self.editor.font = default.editor.font,
			"editor.shaping" => self.editor.shaping = default.editor.shaping,
			"editor.wrapping" => self.editor.wrapping = default.editor.wrapping,
			"editor.font_size" => self.editor.font_size = default.editor.font_size,
			"editor.line_height" => self.editor.line_height = default.editor.line_height,
			"inspect.show_baselines" => self.inspect.show_baselines = default.inspect.show_baselines,
			"inspect.show_hitboxes" => self.inspect.show_hitboxes = default.inspect.show_hitboxes,
			_ => return Err(unknown_path(path)),
		}

		Ok(())
	}

	pub fn value(&self, path: &str) -> Result<GlorpValue, GlorpError> {
		match path {
			"editor.preset" => Ok(self
				.editor
				.preset
				.map_or(GlorpValue::Null, |value| value.as_ref().into())),
			"editor.font" => Ok(self.editor.font.as_ref().into()),
			"editor.shaping" => Ok(self.editor.shaping.as_ref().into()),
			"editor.wrapping" => Ok(self.editor.wrapping.as_ref().into()),
			"editor.font_size" => Ok(self.editor.font_size.into()),
			"editor.line_height" => Ok(self.editor.line_height.into()),
			"inspect.show_baselines" => Ok(self.inspect.show_baselines.into()),
			"inspect.show_hitboxes" => Ok(self.inspect.show_hitboxes.into()),
			_ => Err(unknown_path(path)),
		}
	}

	pub fn validate_path(path: &str, value: GlorpValue) -> Result<(), GlorpError> {
		let mut config = Self::default();
		config.set_path(path, value)
	}

	#[must_use]
	pub fn schema_defaults() -> Vec<(ConfigPath, GlorpValue)> {
		glorp_schema()
			.config
			.into_iter()
			.map(|field| (field.path, field.default))
			.collect()
	}
}

pub trait EnumValue: Copy {
	fn parse(value: &str) -> Option<Self>;
	fn allowed_values() -> &'static [&'static str];
	fn as_ref(self) -> &'static str;
}

impl EnumValue for crate::SamplePreset {
	fn parse(value: &str) -> Option<Self> {
		match value {
			"tall" => Some(Self::Tall),
			"mixed" => Some(Self::Mixed),
			"rust" => Some(Self::Rust),
			"ligatures" => Some(Self::Ligatures),
			"arabic" => Some(Self::Arabic),
			"cjk" => Some(Self::Cjk),
			"emoji" => Some(Self::Emoji),
			"custom" => Some(Self::Custom),
			_ => None,
		}
	}

	fn allowed_values() -> &'static [&'static str] {
		&["tall", "mixed", "rust", "ligatures", "arabic", "cjk", "emoji", "custom"]
	}

	fn as_ref(self) -> &'static str {
		match self {
			Self::Tall => "tall",
			Self::Mixed => "mixed",
			Self::Rust => "rust",
			Self::Ligatures => "ligatures",
			Self::Arabic => "arabic",
			Self::Cjk => "cjk",
			Self::Emoji => "emoji",
			Self::Custom => "custom",
		}
	}
}

impl EnumValue for crate::FontChoice {
	fn parse(value: &str) -> Option<Self> {
		match value {
			"jetbrains-mono" => Some(Self::JetBrainsMono),
			"monospace" => Some(Self::Monospace),
			"noto-sans-cjk" => Some(Self::NotoSansCjk),
			"sans-serif" => Some(Self::SansSerif),
			_ => None,
		}
	}

	fn allowed_values() -> &'static [&'static str] {
		&["jetbrains-mono", "monospace", "noto-sans-cjk", "sans-serif"]
	}

	fn as_ref(self) -> &'static str {
		match self {
			Self::JetBrainsMono => "jetbrains-mono",
			Self::Monospace => "monospace",
			Self::NotoSansCjk => "noto-sans-cjk",
			Self::SansSerif => "sans-serif",
		}
	}
}

impl EnumValue for crate::ShapingChoice {
	fn parse(value: &str) -> Option<Self> {
		match value {
			"auto" => Some(Self::Auto),
			"basic" => Some(Self::Basic),
			"advanced" => Some(Self::Advanced),
			_ => None,
		}
	}

	fn allowed_values() -> &'static [&'static str] {
		&["auto", "basic", "advanced"]
	}

	fn as_ref(self) -> &'static str {
		match self {
			Self::Auto => "auto",
			Self::Basic => "basic",
			Self::Advanced => "advanced",
		}
	}
}

impl EnumValue for crate::WrapChoice {
	fn parse(value: &str) -> Option<Self> {
		match value {
			"none" => Some(Self::None),
			"word" => Some(Self::Word),
			"glyph" => Some(Self::Glyph),
			"word-or-glyph" => Some(Self::WordOrGlyph),
			_ => None,
		}
	}

	fn allowed_values() -> &'static [&'static str] {
		&["none", "word", "glyph", "word-or-glyph"]
	}

	fn as_ref(self) -> &'static str {
		match self {
			Self::None => "none",
			Self::Word => "word",
			Self::Glyph => "glyph",
			Self::WordOrGlyph => "word-or-glyph",
		}
	}
}

fn parse_string_enum<T>(path: &str, value: GlorpValue) -> Result<T, GlorpError>
where
	T: EnumValue, {
	match value {
		GlorpValue::String(value) => parse_enum(path, &value),
		other => Err(type_error(path, "string", other.kind())),
	}
}

fn parse_enum<T>(path: &str, value: &str) -> Result<T, GlorpError>
where
	T: EnumValue, {
	T::parse(value).ok_or_else(|| {
		GlorpError::validation_with_allowed(
			Some(path.to_owned()),
			format!("invalid value `{value}` for `{path}`"),
			T::allowed_values().iter().map(|value| (*value).to_owned()).collect(),
		)
	})
}

fn parse_bool(path: &str, value: &GlorpValue) -> Result<bool, GlorpError> {
	value.as_bool().ok_or_else(|| type_error(path, "bool", value.kind()))
}

fn parse_f32(path: &str, value: &GlorpValue) -> Result<f32, GlorpError> {
	let Some(value) = value.as_f64() else {
		return Err(type_error(path, "float", value.kind()));
	};
	Ok(value as f32)
}

fn type_error(path: &str, expected: &str, actual: &str) -> GlorpError {
	GlorpError::validation(
		Some(path.to_owned()),
		format!("invalid type for `{path}`: expected {expected}, got {actual}"),
	)
}

fn unknown_path(path: &str) -> GlorpError {
	GlorpError::validation(Some(path.to_owned()), format!("unknown config path `{path}`"))
}
