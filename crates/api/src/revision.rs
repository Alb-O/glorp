#[derive(Debug, Clone, Copy, Default, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct GlorpRevisions {
	pub editor: u64,
	pub config: u64,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct GlorpDelta {
	pub text_changed: bool,
	pub view_changed: bool,
	pub selection_changed: bool,
	pub mode_changed: bool,
	pub config_changed: bool,
}
