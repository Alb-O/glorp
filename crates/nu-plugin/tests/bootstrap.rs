use {
	glorp_runtime::{ConfigStore, ConfigStorePaths, RuntimeHost, RuntimeOptions, export_surface_artifacts},
	glorp_transport::{IpcServerHandle, default_socket_path, start_server},
	std::{
		path::PathBuf,
		process::Command,
		time::{SystemTime, UNIX_EPOCH},
	},
};

struct Harness {
	root: PathBuf,
	socket_path: PathBuf,
	paths: ConfigStorePaths,
	server: Option<IpcServerHandle>,
}

impl Harness {
	fn new() -> Self {
		let stamp = SystemTime::now()
			.duration_since(UNIX_EPOCH)
			.expect("current time should be after epoch")
			.as_nanos();
		let root = std::env::temp_dir().join(format!("glorp-plugin-bootstrap-{stamp}"));
		let paths = ConfigStorePaths {
			durable_config_path: root.join("nu/default-config.nu"),
			schema_path: root.join("schema/glorp-schema.json"),
			nu_module_path: root.join("nu/glorp.nu"),
			nu_completions_path: root.join("nu/completions.nu"),
		};
		let socket_path = default_socket_path(&root);

		std::fs::create_dir_all(root.join("nu")).expect("create nu dir");
		std::fs::create_dir_all(root.join("schema")).expect("create schema dir");

		Self {
			root,
			socket_path,
			paths,
			server: None,
		}
	}

	fn runtime(&self) -> RuntimeHost {
		RuntimeHost::new(RuntimeOptions {
			paths: self.paths.clone(),
		})
		.expect("runtime should start")
	}

	fn start_server(&mut self) {
		self.server = Some(start_server(self.socket_path.clone(), self.runtime()).expect("server should start"));
	}

	fn export_surface(&self) {
		export_surface_artifacts(&ConfigStore::new(self.paths.clone())).expect("surface export should succeed");
	}
}

impl Drop for Harness {
	fn drop(&mut self) {
		if let Some(server) = self.server.take() {
			let _ = server.shutdown();
		}
		let _ = std::fs::remove_dir_all(&self.root);
	}
}

fn run_nu(args: &[&str]) -> String {
	let output = Command::new("nu").args(args).output().expect("nu should execute");
	assert!(
		output.status.success(),
		"nu should succeed: stdout=`{}` stderr=`{}`",
		String::from_utf8_lossy(&output.stdout),
		String::from_utf8_lossy(&output.stderr),
	);
	String::from_utf8(output.stdout).expect("nu stdout should be utf8")
}

#[test]
fn sourced_nu_bootstrap_script_controls_runtime_e2e() {
	let mut harness = Harness::new();
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
			r#"source "{}"; glorp exec document-replace {{text: "hello from sourced nu"}} --socket "{}"; glorp query document-text --socket "{}""#,
			harness.paths.nu_module_path.display(),
			harness.socket_path.display(),
			harness.socket_path.display(),
		),
	]);

	assert_eq!(text.trim(), "hello from sourced nu");
}
