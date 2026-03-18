use {
	glorp_api::{
		ConfigAssignment, EditorHistoryCommand, EditorHistoryInput, EditorModeCommand, EditorModeInput, EditorMotion,
		EditorMotionInput, EditorStateView, GlorpCall, GlorpCallDescriptor, GlorpCallResult, GlorpCaller, GlorpConfig,
		GlorpError, GlorpEvent, GlorpOutcome, GlorpValue, StreamTokenInput, TextInput, calls,
	},
	glorp_runtime::{ConfigStore, ConfigStorePaths, RuntimeHost, RuntimeOptions, export_surface_artifacts},
	glorp_transport::{IpcClient, IpcServerHandle, LocalClient, default_socket_path, start_server},
	std::{
		path::PathBuf,
		process::Command,
		time::{SystemTime, UNIX_EPOCH},
	},
};

pub struct TestRepo {
	pub root: PathBuf,
	pub socket_path: PathBuf,
	pub paths: ConfigStorePaths,
	server: Option<IpcServerHandle>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StateSnapshot {
	pub text: String,
	pub editor: EditorStateView,
	pub config: GlorpConfig,
}

impl TestRepo {
	pub fn new(prefix: &str) -> Self {
		let stamp = SystemTime::now()
			.duration_since(UNIX_EPOCH)
			.expect("current time should be after epoch")
			.as_nanos();
		let root = std::env::temp_dir().join(format!("{prefix}-{stamp}"));
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

	pub fn runtime(&self) -> RuntimeHost {
		RuntimeHost::new(RuntimeOptions {
			paths: self.paths.clone(),
		})
		.expect("runtime should start")
	}

	pub fn local_client(&self) -> LocalClient {
		LocalClient::new(self.runtime())
	}

	pub fn start_server(&mut self) {
		self.server = Some(start_server(self.socket_path.clone(), self.runtime()).expect("server should start"));
	}

	pub fn ipc_client(&self) -> IpcClient {
		IpcClient::new(self.socket_path.clone())
	}

	pub fn export_surface(&self) {
		export_surface_artifacts(&ConfigStore::new(self.paths.clone())).expect("surface export should succeed");
	}
}

impl Drop for TestRepo {
	fn drop(&mut self) {
		if let Some(server) = self.server.take() {
			let _ = server.shutdown();
		}
		let _ = std::fs::remove_dir_all(&self.root);
	}
}

pub fn workspace_root() -> PathBuf {
	PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.parent()
		.expect("test-support crate should have a parent")
		.parent()
		.expect("repo root should exist")
		.to_path_buf()
}

pub fn run_nu(args: &[&str]) -> String {
	let output = Command::new("nu").args(args).output().expect("nu should execute");
	assert!(
		output.status.success(),
		"nu should succeed: stdout=`{}` stderr=`{}`",
		String::from_utf8_lossy(&output.stdout),
		String::from_utf8_lossy(&output.stderr),
	);
	String::from_utf8(output.stdout).expect("nu stdout should be utf8")
}

pub fn raw_call_ok(caller: &mut impl GlorpCaller, call: GlorpCall) -> GlorpCallResult {
	caller.call(call).expect("call should succeed")
}

pub fn raw_call_err(caller: &mut impl GlorpCaller, call: GlorpCall) -> GlorpError {
	caller.call(call).expect_err("call should fail")
}

pub fn build_call<D>(input: D::Input) -> GlorpCall
where
	D: GlorpCallDescriptor, {
	D::build(input).expect("call should build")
}

pub fn call_ok<D>(caller: &mut impl GlorpCaller, input: D::Input) -> D::Output
where
	D: GlorpCallDescriptor, {
	D::call(caller, input).expect("call should succeed")
}

pub fn outcome(caller: &mut impl GlorpCaller, call: GlorpCall) -> GlorpOutcome {
	let result = raw_call_ok(caller, call);
	let id = result.id.clone();
	glorp_api::decode_call_output::<GlorpOutcome>(&id, &result.output).expect("outcome payload should decode")
}

pub fn document_text(caller: &mut impl GlorpCaller) -> String {
	call_ok::<calls::DocumentText>(caller, ())
}

pub fn config(caller: &mut impl GlorpCaller) -> GlorpConfig {
	call_ok::<calls::Config>(caller, ())
}

pub fn editor_state(caller: &mut impl GlorpCaller) -> EditorStateView {
	call_ok::<calls::Editor>(caller, ())
}

pub fn state_snapshot(caller: &mut impl GlorpCaller) -> StateSnapshot {
	StateSnapshot {
		text: document_text(caller),
		editor: editor_state(caller),
		config: config(caller),
	}
}

pub fn subscribe_changes(caller: &mut impl GlorpCaller) -> u64 {
	call_ok::<calls::EventsSubscribe>(caller, ()).token
}

pub fn next_event(caller: &mut impl GlorpCaller, token: u64) -> Option<GlorpEvent> {
	call_ok::<calls::EventsNext>(caller, StreamTokenInput { token })
}

pub fn txn(calls: Vec<GlorpCall>) -> GlorpCall {
	build_call::<calls::Txn>(glorp_api::GlorpTxn { calls })
}

pub fn config_set(path: &str, value: GlorpValue) -> GlorpCall {
	build_call::<calls::ConfigSet>(ConfigAssignment {
		path: path.to_owned(),
		value,
	})
}

pub fn document_replace(text: &str) -> GlorpCall {
	build_call::<calls::DocumentReplace>(TextInput { text: text.to_owned() })
}

pub fn editor_mode(mode: EditorModeCommand) -> GlorpCall {
	build_call::<calls::EditorMode>(EditorModeInput { mode })
}

pub fn editor_motion(motion: EditorMotion) -> GlorpCall {
	build_call::<calls::EditorMotion>(EditorMotionInput { motion })
}

pub fn editor_insert(text: &str) -> GlorpCall {
	build_call::<calls::EditorInsert>(TextInput { text: text.to_owned() })
}

pub fn editor_history(action: EditorHistoryCommand) -> GlorpCall {
	build_call::<calls::EditorHistory>(EditorHistoryInput { action })
}

pub fn run_standard_transcript(caller: &mut impl GlorpCaller) -> StateSnapshot {
	let _ = outcome(
		caller,
		config_set("editor.wrapping", GlorpValue::String("word".to_owned())),
	);
	let _ = outcome(caller, document_replace("hello"));
	let _ = outcome(caller, editor_mode(EditorModeCommand::EnterInsertAfter));
	let _ = outcome(caller, editor_motion(EditorMotion::LineEnd));
	let _ = outcome(caller, editor_insert(" world"));

	state_snapshot(caller)
}
