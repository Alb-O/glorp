use {
	glorp_test_support::{TestRepo, run_nu},
	std::path::PathBuf,
};

#[test]
fn sourced_nu_bootstrap_script_controls_runtime_e2e() {
	let mut harness = TestRepo::new("glorp-plugin-bootstrap");
	harness.export_surface();
	harness.start_server();

	let plugin_config = harness.root.join("plugins.msgpackz");
	let plugin_bin = PathBuf::from(env!("CARGO_BIN_EXE_nu_plugin_glorp"));

	let _ = run_nu(&[
		"-c",
		&format!(
			r#"plugin add --plugin-config "{}" "{}""#,
			plugin_config.display(),
			plugin_bin.display(),
		),
	]);

	let text = run_nu(&[
		"--plugin-config",
		plugin_config.to_str().expect("plugin config should be utf8"),
		"-c",
		&format!(
			r#"source "{}"; glorp call document-replace {{text: "hello from sourced nu"}} --socket "{}"; glorp call document-text --socket "{}""#,
			harness.paths.nu_module_path.display(),
			harness.socket_path.display(),
			harness.socket_path.display(),
		),
	]);

	assert_eq!(text.trim(), "hello from sourced nu");
}
