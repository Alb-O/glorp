use crate::ConfigPath;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq, thiserror::Error)]
pub enum GlorpError {
	#[error("{message}")]
	Validation {
		path: Option<ConfigPath>,
		message: String,
		allowed_values: Vec<String>,
	},
	#[error("{message}")]
	NotFound { message: String },
	#[error("{message}")]
	Transport { message: String },
	#[error("{message}")]
	Internal { message: String },
}

impl GlorpError {
	#[must_use]
	pub fn validation(path: impl Into<Option<ConfigPath>>, message: impl Into<String>) -> Self {
		Self::Validation {
			path: path.into(),
			message: message.into(),
			allowed_values: Vec::new(),
		}
	}

	#[must_use]
	pub fn validation_with_allowed(
		path: impl Into<Option<ConfigPath>>, message: impl Into<String>, allowed_values: Vec<String>,
	) -> Self {
		Self::Validation {
			path: path.into(),
			message: message.into(),
			allowed_values,
		}
	}

	#[must_use]
	pub fn internal(message: impl Into<String>) -> Self {
		Self::Internal {
			message: message.into(),
		}
	}

	#[must_use]
	pub fn transport(message: impl Into<String>) -> Self {
		Self::Transport {
			message: message.into(),
		}
	}

	#[must_use]
	pub fn not_found(message: impl Into<String>) -> Self {
		Self::NotFound {
			message: message.into(),
		}
	}
}
