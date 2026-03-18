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
			.try_fold(Vec::with_capacity(values.len()), |mut paths, assignment| {
				self.set_path(&assignment.path, &assignment.value)?;
				paths.push(assignment.path.clone());
				Ok(paths)
			})
	}

	pub fn set_path(&mut self, path: &str, value: &GlorpValue) -> Result<(), GlorpError> {
		match path {
			"editor.preset" => self.editor.preset = parse_optional_string_enum(path, value)?,
			"editor.font" => self.editor.font = parse_string_enum(path, value)?,
			"editor.shaping" => self.editor.shaping = parse_string_enum(path, value)?,
			"editor.wrapping" => self.editor.wrapping = parse_string_enum(path, value)?,
			"editor.font_size" => self.editor.font_size = parse_f32(path, value)?,
			"editor.line_height" => self.editor.line_height = parse_f32(path, value)?,
			"inspect.show_baselines" => self.inspect.show_baselines = parse_bool(path, value)?,
			"inspect.show_hitboxes" => self.inspect.show_hitboxes = parse_bool(path, value)?,
			_ => return Err(unknown_path(path)),
		}

		Ok(())
	}

	pub fn reset_path(&mut self, path: &str) -> Result<(), GlorpError> {
		let value = Self::default().value(path)?;
		self.set_path(path, &value)
	}

	pub fn value(&self, path: &str) -> Result<GlorpValue, GlorpError> {
		match path {
			"editor.preset" => Ok(optional_enum_value(self.editor.preset)),
			"editor.font" => Ok(enum_value(self.editor.font)),
			"editor.shaping" => Ok(enum_value(self.editor.shaping)),
			"editor.wrapping" => Ok(enum_value(self.editor.wrapping)),
			"editor.font_size" => Ok(self.editor.font_size.into()),
			"editor.line_height" => Ok(self.editor.line_height.into()),
			"inspect.show_baselines" => Ok(self.inspect.show_baselines.into()),
			"inspect.show_hitboxes" => Ok(self.inspect.show_hitboxes.into()),
			_ => Err(unknown_path(path)),
		}
	}

	pub fn validate_path(path: &str, value: GlorpValue) -> Result<(), GlorpError> {
		let mut config = Self::default();
		config.set_path(path, &value)
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

macro_rules! impl_enum_value {
	($ty:path { $($value:literal => $variant:path),+ $(,)? }) => {
		impl EnumValue for $ty {
			fn parse(value: &str) -> Option<Self> {
				match value {
					$($value => Some($variant),)+
					_ => None,
				}
			}

			fn allowed_values() -> &'static [&'static str] {
				&[$($value),+]
			}

			fn as_ref(self) -> &'static str {
				match self {
					$($variant => $value),+
				}
			}
		}
	};
}

impl_enum_value!(crate::SamplePreset {
	"tall" => crate::SamplePreset::Tall,
	"mixed" => crate::SamplePreset::Mixed,
	"rust" => crate::SamplePreset::Rust,
	"ligatures" => crate::SamplePreset::Ligatures,
	"arabic" => crate::SamplePreset::Arabic,
	"cjk" => crate::SamplePreset::Cjk,
	"emoji" => crate::SamplePreset::Emoji,
	"custom" => crate::SamplePreset::Custom,
});

impl_enum_value!(crate::FontChoice {
	"jetbrains-mono" => crate::FontChoice::JetBrainsMono,
	"monospace" => crate::FontChoice::Monospace,
	"noto-sans-cjk" => crate::FontChoice::NotoSansCjk,
	"sans-serif" => crate::FontChoice::SansSerif,
});

impl_enum_value!(crate::ShapingChoice {
	"auto" => crate::ShapingChoice::Auto,
	"basic" => crate::ShapingChoice::Basic,
	"advanced" => crate::ShapingChoice::Advanced,
});

impl_enum_value!(crate::WrapChoice {
	"none" => crate::WrapChoice::None,
	"word" => crate::WrapChoice::Word,
	"glyph" => crate::WrapChoice::Glyph,
	"word-or-glyph" => crate::WrapChoice::WordOrGlyph,
});

impl_enum_value!(crate::EditorMotion {
	"left" => crate::EditorMotion::Left,
	"right" => crate::EditorMotion::Right,
	"up" => crate::EditorMotion::Up,
	"down" => crate::EditorMotion::Down,
	"line-start" => crate::EditorMotion::LineStart,
	"line-end" => crate::EditorMotion::LineEnd,
});

impl_enum_value!(crate::EditorModeCommand {
	"enter-insert-before" => crate::EditorModeCommand::EnterInsertBefore,
	"enter-insert-after" => crate::EditorModeCommand::EnterInsertAfter,
	"exit-insert" => crate::EditorModeCommand::ExitInsert,
});

impl_enum_value!(crate::EditorHistoryCommand {
	"undo" => crate::EditorHistoryCommand::Undo,
	"redo" => crate::EditorHistoryCommand::Redo,
});

impl_enum_value!(crate::SidebarTab {
	"controls" => crate::SidebarTab::Controls,
	"inspect" => crate::SidebarTab::Inspect,
	"perf" => crate::SidebarTab::Perf,
});

fn parse_string_enum<T>(path: &str, value: &GlorpValue) -> Result<T, GlorpError>
where
	T: EnumValue, {
	value
		.as_str()
		.ok_or_else(|| type_error(path, "string", value.kind()))
		.and_then(|value| parse_enum(path, value))
}

fn parse_optional_string_enum<T>(path: &str, value: &GlorpValue) -> Result<Option<T>, GlorpError>
where
	T: EnumValue, {
	match value {
		GlorpValue::Null => Ok(None),
		_ => parse_string_enum(path, value).map(Some),
	}
}

fn parse_enum<T>(path: &str, value: &str) -> Result<T, GlorpError>
where
	T: EnumValue, {
	T::parse(value).ok_or_else(|| {
		GlorpError::validation_with_allowed(
			Some(path.to_owned()),
			format!("invalid value `{value}` for `{path}`"),
			T::allowed_values().iter().copied().map(str::to_owned).collect(),
		)
	})
}

fn enum_value<T>(value: T) -> GlorpValue
where
	T: EnumValue, {
	value.as_ref().into()
}

fn optional_enum_value<T>(value: Option<T>) -> GlorpValue
where
	T: EnumValue, {
	value.map_or(GlorpValue::Null, enum_value)
}

fn parse_bool(path: &str, value: &GlorpValue) -> Result<bool, GlorpError> {
	value.as_bool().ok_or_else(|| type_error(path, "bool", value.kind()))
}

fn parse_f32(path: &str, value: &GlorpValue) -> Result<f32, GlorpError> {
	value
		.as_f64()
		.map(|value| value as f32)
		.ok_or_else(|| type_error(path, "float", value.kind()))
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
