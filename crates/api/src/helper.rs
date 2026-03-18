use crate::{ConfigAssignment, GlorpEvent, GlorpEventStreamView, GlorpSessionView};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(tag = "op", content = "input", rename_all = "kebab-case")]
pub enum GlorpHelper {
	SessionAttach,
	SessionShutdown,
	ConfigValidate(ConfigAssignment),
	EventsSubscribe,
	EventsNext(StreamTokenInput),
	EventsUnsubscribe(StreamTokenInput),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(tag = "kind", content = "output", rename_all = "kebab-case")]
pub enum GlorpHelperResult {
	SessionAttach(GlorpSessionView),
	SessionShutdown(OkView),
	ConfigValidate(OkView),
	EventsSubscribe(GlorpEventStreamView),
	EventsNext(Option<GlorpEvent>),
	EventsUnsubscribe(TokenAckView),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct StreamTokenInput {
	pub token: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct OkView {
	pub ok: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct TokenAckView {
	pub ok: bool,
	pub token: u64,
}
