use crate::{BuiltinType, ConfigAssignment, ConfigFieldSchema, ConfigPath, GlorpError, GlorpValue, TypeRef};

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

macro_rules! config_fields {
	($field:ident) => {
		$field!(
			"editor.preset",
			"Optional sample preset.",
			TypeRef::Named("SamplePreset".to_owned()),
			GlorpValue::String("tall".into())
		);
		$field!(
			"editor.font",
			"Editor font choice.",
			TypeRef::Named("FontChoice".to_owned()),
			GlorpValue::String("jetbrains-mono".into())
		);
		$field!(
			"editor.shaping",
			"Editor shaping mode.",
			TypeRef::Named("ShapingChoice".to_owned()),
			GlorpValue::String("advanced".into())
		);
		$field!(
			"editor.wrapping",
			"Editor wrapping mode.",
			TypeRef::Named("WrapChoice".to_owned()),
			GlorpValue::String("word".into())
		);
		$field!(
			"editor.font_size",
			"Editor font size in logical pixels.",
			TypeRef::Builtin(BuiltinType::Float),
			24.0.into()
		);
		$field!(
			"editor.line_height",
			"Editor line height in logical pixels.",
			TypeRef::Builtin(BuiltinType::Float),
			32.0.into()
		);
		$field!(
			"inspect.show_baselines",
			"Show line baselines in inspect mode.",
			TypeRef::Builtin(BuiltinType::Bool),
			false.into()
		);
		$field!(
			"inspect.show_hitboxes",
			"Show glyph hitboxes in inspect mode.",
			TypeRef::Builtin(BuiltinType::Bool),
			false.into()
		);
	};
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
			"editor.preset" => {
				self.editor.preset = parse_optional_string_enum(path, value)?;
				Ok(())
			}
			"editor.font" => {
				self.editor.font = parse_string_enum(path, value)?;
				Ok(())
			}
			"editor.shaping" => {
				self.editor.shaping = parse_string_enum(path, value)?;
				Ok(())
			}
			"editor.wrapping" => {
				self.editor.wrapping = parse_string_enum(path, value)?;
				Ok(())
			}
			"editor.font_size" => {
				self.editor.font_size = parse_f32(path, value)?;
				Ok(())
			}
			"editor.line_height" => {
				self.editor.line_height = parse_f32(path, value)?;
				Ok(())
			}
			"inspect.show_baselines" => {
				self.inspect.show_baselines = parse_bool(path, value)?;
				Ok(())
			}
			"inspect.show_hitboxes" => {
				self.inspect.show_hitboxes = parse_bool(path, value)?;
				Ok(())
			}
			_ => Err(unknown_path(path)),
		}
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
		config_schema_fields()
			.into_iter()
			.map(|field| (field.path, field.default))
			.collect()
	}
}

#[must_use]
pub fn config_schema_fields() -> Vec<ConfigFieldSchema> {
	let mut fields = Vec::new();

	macro_rules! schema_field {
		($field_path:literal, $docs:literal, $ty:expr, $default:expr) => {
			fields.push(config_field($field_path, $docs, $ty, $default));
		};
	}

	config_fields!(schema_field);
	fields
}

fn config_field(path: &str, docs: &str, ty: TypeRef, default: GlorpValue) -> ConfigFieldSchema {
	ConfigFieldSchema {
		path: path.to_owned(),
		docs: docs.to_owned(),
		ty,
		default,
		mutable: true,
	}
}

pub trait EnumValue: Copy {
	fn parse(value: &str) -> Option<Self>;
	fn allowed_values() -> &'static [&'static str];
	fn docs(value: &str) -> Option<&'static str>;
	fn as_ref(self) -> &'static str;
}

macro_rules! impl_enum_value {
	($ty:path { $($value:literal => $variant:path : $docs:literal),+ $(,)? }) => {
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

			fn docs(value: &str) -> Option<&'static str> {
				match value {
					$($value => Some($docs),)+
					_ => None,
				}
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
	"tall" => crate::SamplePreset::Tall: "A tall multi-script sample.",
	"mixed" => crate::SamplePreset::Mixed: "A short mixed-script sample.",
	"rust" => crate::SamplePreset::Rust: "A Rust source sample.",
	"ligatures" => crate::SamplePreset::Ligatures: "A ligature-heavy sample.",
	"arabic" => crate::SamplePreset::Arabic: "An Arabic sample.",
	"cjk" => crate::SamplePreset::Cjk: "A CJK sample.",
	"emoji" => crate::SamplePreset::Emoji: "An emoji-heavy sample.",
	"custom" => crate::SamplePreset::Custom: "No built-in sample.",
});

impl_enum_value!(crate::FontChoice {
	"jetbrains-mono" => crate::FontChoice::JetBrainsMono: "JetBrains Mono.",
	"monospace" => crate::FontChoice::Monospace: "The platform monospace family.",
	"noto-sans-cjk" => crate::FontChoice::NotoSansCjk: "Noto Sans CJK.",
	"sans-serif" => crate::FontChoice::SansSerif: "The platform sans-serif family.",
});

impl_enum_value!(crate::ShapingChoice {
	"auto" => crate::ShapingChoice::Auto: "Choose shaping based on content.",
	"basic" => crate::ShapingChoice::Basic: "Use basic shaping.",
	"advanced" => crate::ShapingChoice::Advanced: "Use advanced shaping.",
});

impl_enum_value!(crate::WrapChoice {
	"none" => crate::WrapChoice::None: "Do not wrap lines.",
	"word" => crate::WrapChoice::Word: "Wrap at word boundaries.",
	"glyph" => crate::WrapChoice::Glyph: "Wrap at glyph boundaries.",
	"word-or-glyph" => crate::WrapChoice::WordOrGlyph: "Prefer word boundaries, fall back to glyph boundaries.",
});

impl_enum_value!(crate::EditorMotion {
	"left" => crate::EditorMotion::Left: "Move left.",
	"right" => crate::EditorMotion::Right: "Move right.",
	"up" => crate::EditorMotion::Up: "Move up.",
	"down" => crate::EditorMotion::Down: "Move down.",
	"line-start" => crate::EditorMotion::LineStart: "Move to line start.",
	"line-end" => crate::EditorMotion::LineEnd: "Move to line end.",
});

impl_enum_value!(crate::EditorModeCommand {
	"enter-insert-before" => crate::EditorModeCommand::EnterInsertBefore: "Enter insert mode before the selection.",
	"enter-insert-after" => crate::EditorModeCommand::EnterInsertAfter: "Enter insert mode after the selection.",
	"exit-insert" => crate::EditorModeCommand::ExitInsert: "Return to normal mode.",
});

impl_enum_value!(crate::EditorHistoryCommand {
	"undo" => crate::EditorHistoryCommand::Undo: "Undo the most recent edit.",
	"redo" => crate::EditorHistoryCommand::Redo: "Redo the most recent undone edit.",
});

impl_enum_value!(crate::EditorMode {
	"normal" => crate::EditorMode::Normal: "Normal mode.",
	"insert" => crate::EditorMode::Insert: "Insert mode.",
});

impl_enum_value!(crate::SidebarTab {
	"controls" => crate::SidebarTab::Controls: "Configuration controls.",
	"inspect" => crate::SidebarTab::Inspect: "Scene inspection.",
	"perf" => crate::SidebarTab::Perf: "Performance projections.",
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
