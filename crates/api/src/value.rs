use std::collections::BTreeMap;

pub type ConfigPath = String;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct ConfigAssignment {
	pub path: ConfigPath,
	pub value: GlorpValue,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(untagged)]
pub enum GlorpValue {
	Null,
	Bool(bool),
	Int(i64),
	Float(f64),
	String(String),
	List(Vec<Self>),
	Record(BTreeMap<String, Self>),
}

impl GlorpValue {
	#[must_use]
	pub const fn kind(&self) -> &'static str {
		match self {
			Self::Null => "null",
			Self::Bool(_) => "bool",
			Self::Int(_) => "int",
			Self::Float(_) => "float",
			Self::String(_) => "string",
			Self::List(_) => "list",
			Self::Record(_) => "record",
		}
	}

	#[must_use]
	pub const fn as_bool(&self) -> Option<bool> {
		match self {
			Self::Bool(value) => Some(*value),
			_ => None,
		}
	}

	#[must_use]
	pub const fn as_i64(&self) -> Option<i64> {
		match self {
			Self::Int(value) => Some(*value),
			_ => None,
		}
	}

	#[must_use]
	pub const fn as_f64(&self) -> Option<f64> {
		match self {
			Self::Float(value) => Some(*value),
			Self::Int(value) => Some(*value as f64),
			_ => None,
		}
	}

	#[must_use]
	pub fn as_str(&self) -> Option<&str> {
		match self {
			Self::String(value) => Some(value),
			_ => None,
		}
	}
}

impl From<bool> for GlorpValue {
	fn from(value: bool) -> Self {
		Self::Bool(value)
	}
}

impl From<i64> for GlorpValue {
	fn from(value: i64) -> Self {
		Self::Int(value)
	}
}

impl From<f32> for GlorpValue {
	fn from(value: f32) -> Self {
		Self::Float(f64::from(value))
	}
}

impl From<f64> for GlorpValue {
	fn from(value: f64) -> Self {
		Self::Float(value)
	}
}

impl From<String> for GlorpValue {
	fn from(value: String) -> Self {
		Self::String(value)
	}
}

impl From<&str> for GlorpValue {
	fn from(value: &str) -> Self {
		Self::String(value.into())
	}
}

impl From<serde_json::Value> for GlorpValue {
	fn from(value: serde_json::Value) -> Self {
		match value {
			serde_json::Value::Null => Self::Null,
			serde_json::Value::Bool(value) => Self::Bool(value),
			serde_json::Value::Number(value) => value
				.as_i64()
				.map(Self::Int)
				.or_else(|| value.as_f64().map(Self::Float))
				.unwrap_or(Self::Null),
			serde_json::Value::String(value) => Self::String(value),
			serde_json::Value::Array(values) => Self::List(values.into_iter().map(Into::into).collect()),
			serde_json::Value::Object(values) => {
				Self::Record(values.into_iter().map(|(key, value)| (key, value.into())).collect())
			}
		}
	}
}

impl From<GlorpValue> for serde_json::Value {
	fn from(value: GlorpValue) -> Self {
		match value {
			GlorpValue::Null => Self::Null,
			GlorpValue::Bool(value) => Self::Bool(value),
			GlorpValue::Int(value) => Self::Number(value.into()),
			GlorpValue::Float(value) => serde_json::json!(value),
			GlorpValue::String(value) => Self::String(value),
			GlorpValue::List(values) => Self::Array(values.into_iter().map(Into::into).collect()),
			GlorpValue::Record(values) => {
				Self::Object(values.into_iter().map(|(key, value)| (key, value.into())).collect())
			}
		}
	}
}
