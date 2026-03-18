use crate::GlorpValue;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct GlorpTxn {
	pub commands: Vec<GlorpInvocation>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct GlorpInvocation {
	pub path: String,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub input: Option<GlorpValue>,
}
