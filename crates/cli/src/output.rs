use glorp_api::{GlorpOutcome, GlorpQueryResult};

pub fn print_query(result: &GlorpQueryResult) -> Result<(), glorp_api::GlorpError> {
	match result {
		GlorpQueryResult::Schema(value) => print_json(value),
		GlorpQueryResult::Config(value) => print_json(value),
		GlorpQueryResult::Snapshot(value) => print_json(value),
		GlorpQueryResult::DocumentText(value) => print_json(value),
		GlorpQueryResult::Capabilities(value) => print_json(value),
	}
}

pub fn print_outcome(outcome: &GlorpOutcome) -> Result<(), glorp_api::GlorpError> {
	print_json(outcome)
}

pub fn print_json<T>(value: &T) -> Result<(), glorp_api::GlorpError>
where
	T: serde::Serialize, {
	let json = serde_json::to_string_pretty(value)
		.map_err(|error| glorp_api::GlorpError::internal(format!("failed to encode output: {error}")))?;
	println!("{json}");
	Ok(())
}
