use crate::{ConfigPath, GlorpDelta, GlorpRevisions};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", content = "payload", rename_all = "kebab-case")]
pub enum GlorpEvent {
	Changed(GlorpOutcome),
	Notice(GlorpNotice),
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct GlorpOutcome {
	pub delta: GlorpDelta,
	pub revisions: GlorpRevisions,
	pub document_edit: Option<TextEditView>,
	pub changed_config_paths: Vec<ConfigPath>,
	pub warnings: Vec<GlorpWarning>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct TextEditView {
	pub range: crate::TextRange,
	pub inserted: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct GlorpWarning {
	pub code: String,
	pub message: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct GlorpNotice {
	pub code: String,
	pub message: String,
}
