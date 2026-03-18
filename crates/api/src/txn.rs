#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct GlorpTxn {
	pub commands: Vec<crate::GlorpCommand>,
}
