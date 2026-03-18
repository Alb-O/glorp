#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct GlorpTxn {
	pub execs: Vec<crate::GlorpExec>,
}
