fn main() -> Result<(), Box<dyn std::error::Error>> {
	let changed = glorp_api_codegen::write_generated_calls()?;
	if changed {
		println!("updated {}", glorp_api_codegen::generated_calls_path().display());
	} else {
		println!("generated_calls.rs already up to date");
	}
	Ok(())
}
