use {
	glorp_api::*,
	glorp_gui::{GuiLaunchOptions, GuiRuntimeSession},
	glorp_nu_plugin::GlorpPlugin,
	glorp_runtime::{
		ConfigStore, ConfigStorePaths, GuiCommand, RuntimeHost, RuntimeOptions, SidebarTab, export_surface_artifacts,
	},
	glorp_transport::{IpcClient, IpcServerHandle, default_socket_path, start_server},
	nu_plugin_test_support::PluginTest,
	nu_protocol::{Span, Value},
	std::{
		path::PathBuf,
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
		let root = std::env::temp_dir().join(format!("glorp-acceptance-{stamp}"));
		let paths = ConfigStorePaths {
			durable_config_path: root.join("nu/default-config.nu"),
			schema_path: root.join("schema/glorp-schema.json"),
			nu_module_path: root.join("nu/glorp.nu"),
			nu_completions_path: root.join("nu/completions.nu"),
		};
		let socket_path = root.join("glorp.sock");

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

	fn ipc_client(&self) -> IpcClient {
		IpcClient::new(self.socket_path.clone())
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

fn eval_to_value(plugin_test: &mut PluginTest, nu_source: &str) -> Value {
	plugin_test
		.eval(nu_source)
		.expect("Nushell evaluation should succeed")
		.into_value(Span::test_data())
		.expect("pipeline should convert to a value")
}

fn call(host: &mut impl GlorpCaller, glorp_call: GlorpCall) -> GlorpCallResult {
	host.call(glorp_call).expect("call should succeed")
}

fn outcome(host: &mut impl GlorpCaller, glorp_call: GlorpCall) -> GlorpOutcome {
	let result = call(host, glorp_call);
	let id = result.id.clone();
	decode_call_output::<GlorpOutcome>(&id, &result.output).expect("outcome payload should decode")
}

fn document_text(host: &mut impl GlorpCaller) -> String {
	calls::DocumentText::call(host, ()).expect("document-text should succeed")
}

fn config(host: &mut impl GlorpCaller) -> GlorpConfig {
	calls::Config::call(host, ()).expect("config should succeed")
}

fn editor_state(host: &mut impl GlorpCaller) -> EditorStateView {
	calls::Editor::call(host, ()).expect("editor should succeed")
}

fn host_state(host: &mut impl GlorpCaller) -> (String, EditorStateView, GlorpConfig) {
	(document_text(host), editor_state(host), config(host))
}

fn string_field(value: &Value, field: &str) -> String {
	value
		.get_data_by_key(field)
		.and_then(|value| value.coerce_str().ok().map(std::borrow::Cow::into_owned))
		.unwrap_or_else(|| panic!("{field} field should be string"))
}

fn int_field(value: &Value, field: &str) -> i64 {
	value
		.get_data_by_key(field)
		.and_then(|value| match value {
			Value::Int { val, .. } => Some(val),
			_ => None,
		})
		.unwrap_or_else(|| panic!("{field} field should be int"))
}

fn subscribe_changes(host: &mut impl GlorpCaller) -> u64 {
	calls::EventsSubscribe::call(host, ())
		.expect("events-subscribe should succeed")
		.token
}

fn next_event(host: &mut impl GlorpCaller, token: u64) -> Option<GlorpEvent> {
	calls::EventsNext::call(host, StreamTokenInput { token }).expect("events-next should succeed")
}

fn txn(calls: Vec<GlorpCall>) -> GlorpCall {
	calls::Txn::build(GlorpTxn { calls }).expect("txn should build")
}

fn config_set(path: &str, value: GlorpValue) -> GlorpCall {
	calls::ConfigSet::build(ConfigAssignment {
		path: path.to_owned(),
		value,
	})
	.expect("config-set should build")
}

fn document_replace(text: &str) -> GlorpCall {
	calls::DocumentReplace::build(TextInput { text: text.to_owned() }).expect("document-replace should build")
}

fn editor_mode(mode: EditorModeCommand) -> GlorpCall {
	calls::EditorMode::build(EditorModeInput { mode }).expect("editor-mode should build")
}

fn editor_motion(motion: EditorMotion) -> GlorpCall {
	calls::EditorMotion::build(EditorMotionInput { motion }).expect("editor-motion should build")
}

fn editor_insert(text: &str) -> GlorpCall {
	calls::EditorInsert::build(TextInput { text: text.to_owned() }).expect("editor-insert should build")
}

fn editor_history(action: EditorHistoryCommand) -> GlorpCall {
	calls::EditorHistory::build(EditorHistoryInput { action }).expect("editor-history should build")
}

fn run_standard_transcript(host: &mut impl GlorpCaller) -> (String, EditorStateView, GlorpConfig) {
	let _ = outcome(
		host,
		config_set("editor.wrapping", GlorpValue::String("word".to_owned())),
	);
	let _ = outcome(host, document_replace("hello"));
	let _ = outcome(host, editor_mode(EditorModeCommand::EnterInsertAfter));
	let _ = outcome(host, editor_motion(EditorMotion::LineEnd));
	let _ = outcome(host, editor_insert(" world"));

	host_state(host)
}

fn repo_root() -> PathBuf {
	PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.parent()
		.expect("host crate should have a parent")
		.parent()
		.expect("repo root should exist")
		.to_path_buf()
}

fn host_bin() -> PathBuf {
	PathBuf::from(env!("CARGO_BIN_EXE_glorp_host"))
}

fn gui_options(harness: &Harness) -> GuiLaunchOptions {
	GuiLaunchOptions {
		repo_root: harness.root.clone(),
		socket_path: default_socket_path(&harness.root),
	}
}

fn set_host_bin() {
	unsafe {
		std::env::set_var("GLORP_HOST_BIN", host_bin());
	}
}

#[test]
fn schema_export_smoke_test() {
	let harness = Harness::new();
	harness.export_surface();
	let mut host = harness.runtime();

	let schema = calls::Schema::call(&mut host, ()).expect("schema should succeed");

	assert_eq!(schema.version, 6);
	assert!(schema.calls.iter().any(|operation| operation.id == "editor"));
	assert!(schema.calls.iter().all(|operation| operation.id != "snapshot"));
	assert!(schema.calls.iter().all(|operation| operation.id != "selection"));
	assert!(schema.calls.iter().all(|operation| operation.id != "scene-ensure"));
	assert!(schema.calls.iter().all(|operation| operation.id != "ui-sidebar-select"));
	assert!(
		schema
			.calls
			.iter()
			.all(|operation| operation.id != "editor-pointer-begin")
	);
	assert!(schema.types.iter().any(|ty| ty.name == "EditorStateView"));
	assert!(schema.types.iter().all(|ty| ty.name != "GlorpSnapshot"));
	assert!(schema.types.iter().all(|ty| ty.name != "InspectConfig"));
	assert!(harness.paths.schema_path.exists());
}

#[test]
fn nu_plugin_roundtrip_smoke_test() {
	let mut harness = Harness::new();
	harness.start_server();

	let mut plugin_test = PluginTest::new("glorp", GlorpPlugin.into()).expect("plugin test should build");
	let before = eval_to_value(
		&mut plugin_test,
		&format!(r#"glorp call config --socket "{}""#, harness.socket_path.display()),
	);
	let _ = eval_to_value(
		&mut plugin_test,
		&format!(
			r#"glorp call config-set {{path: "editor.wrapping", value: "glyph"}} --socket "{}""#,
			harness.socket_path.display()
		),
	);
	let after = eval_to_value(
		&mut plugin_test,
		&format!(r#"glorp call config --socket "{}""#, harness.socket_path.display()),
	);

	assert_ne!(before, after);
}

#[test]
fn invalid_config_rejection_e2e() {
	let mut host = Harness::new().runtime();
	let before_text = document_text(&mut host);
	let before_editor = editor_state(&mut host);
	let before_config = calls::Config::call(&mut host, ()).expect("config should succeed");

	let error = host
		.call(config_set(
			"editor.wrapping",
			GlorpValue::String("definitely-not-valid".to_owned()),
		))
		.expect_err("invalid config should fail");

	match error {
		GlorpError::Validation {
			path, allowed_values, ..
		} => {
			assert_eq!(path.as_deref(), Some("editor.wrapping"));
			assert!(allowed_values.contains(&"word".to_owned()));
		}
		other => panic!("unexpected error: {other:?}"),
	}

	assert_eq!(before_text, document_text(&mut host));
	assert_eq!(before_editor, editor_state(&mut host));
	let after_config = calls::Config::call(&mut host, ()).expect("config should succeed");
	assert_eq!(before_config, after_config);
}

#[test]
fn transaction_atomicity_e2e() {
	let mut host = Harness::new().runtime();
	let token = subscribe_changes(&mut host);
	let before_text = document_text(&mut host);
	let before_editor = editor_state(&mut host);

	let error = host
		.call(txn(vec![
			config_set("editor.wrapping", GlorpValue::String("glyph".into())),
			document_replace("changed"),
			config_set("editor.wrapping", GlorpValue::String("invalid-value".into())),
		]))
		.expect_err("transaction should fail");
	assert!(matches!(error, GlorpError::Validation { .. }));

	assert_eq!(before_text, document_text(&mut host));
	assert_eq!(before_editor, editor_state(&mut host));
	assert!(next_event(&mut host, token).is_none());
}

#[test]
fn nested_transaction_rejection_e2e() {
	let mut host = Harness::new().runtime();
	let error = host
		.call(txn(vec![txn(vec![document_replace("nested")])]))
		.expect_err("nested transaction should fail");

	match error {
		GlorpError::Validation { message, .. } => assert!(message.contains("nested transactions")),
		other => panic!("unexpected error: {other:?}"),
	}
}

#[test]
fn transaction_rejects_non_transactional_and_non_runtime_calls_e2e() {
	let mut host = Harness::new().runtime();

	for nested_call in [
		calls::Schema::build(()).expect("schema should build"),
		calls::SessionShutdown::build(()).expect("session-shutdown should build"),
	] {
		let error = host.call(txn(vec![nested_call])).expect_err("txn should fail");
		match error {
			GlorpError::Validation { message, .. } => assert!(message.contains("not allowed inside `txn`")),
			other => panic!("unexpected error: {other:?}"),
		}
	}
}

#[test]
fn runtime_dispatch_rejects_non_runtime_routes_e2e() {
	let mut host = Harness::new().runtime();

	for call in [
		calls::SessionAttach::build(()).expect("session-attach should build"),
		calls::SessionShutdown::build(()).expect("session-shutdown should build"),
	] {
		let error = host.call(call).expect_err("runtime should reject non-runtime routes");
		match error {
			GlorpError::Validation { message, .. } => assert!(message.contains("route")),
			other => panic!("unexpected error: {other:?}"),
		}
	}
}

#[test]
fn ipc_transport_rejects_client_route_calls_e2e() {
	let mut harness = Harness::new();
	harness.start_server();
	let mut client = harness.ipc_client();

	let error = client
		.call(calls::SessionAttach::build(()).expect("session-attach should build"))
		.expect_err("client route should be rejected over IPC");

	match error {
		GlorpError::Validation { message, .. } => assert!(message.contains("client route")),
		other => panic!("unexpected error: {other:?}"),
	}
}

#[test]
fn private_gui_state_does_not_emit_public_events() {
	let mut host = Harness::new().runtime();
	let token = subscribe_changes(&mut host);

	host.execute_gui(GuiCommand::SidebarSelect(SidebarTab::Inspect))
		.expect("private sidebar update should succeed");
	host.execute_gui(GuiCommand::ViewportScrollTo { x: 0.0, y: 120.0 })
		.expect("private scroll update should succeed");

	let frame = host.gui_frame();
	assert_eq!(frame.ui.active_tab, SidebarTab::Inspect);
	assert!((frame.ui.canvas_scroll_y - 120.0).abs() <= f32::EPSILON);
	assert!(next_event(&mut host, token).is_none());
}

#[test]
fn private_viewport_resize_updates_public_editor_state() {
	let mut host = Harness::new().runtime();
	let before = editor_state(&mut host);
	let token = subscribe_changes(&mut host);

	host.execute_gui(GuiCommand::ViewportMetricsSet {
		layout_width: 240.0,
		viewport_width: 240.0,
		viewport_height: 200.0,
	})
	.expect("private viewport resize should succeed");

	let after = editor_state(&mut host);
	assert_ne!(before.revisions.editor, after.revisions.editor);
	assert!(after.viewport.measured_width > 0.0);
	let Some(GlorpEvent::Changed(event)) = next_event(&mut host, token) else {
		panic!("expected one public changed event");
	};
	assert!(event.delta.view_changed);
}

#[test]
fn gui_launcher_socket_contract_e2e() {
	let mut harness = Harness::new();
	let options = gui_options(&harness);
	let (mut launched, mut launched_client) =
		GuiRuntimeSession::connect_or_start(options.clone()).expect("launcher should start runtime");
	assert!(launched.owns_server());
	let _ = outcome(&mut launched_client, document_replace("launched"));
	assert_eq!(document_text(&mut launched_client), "launched");
	launched.shutdown().expect("launcher shutdown should succeed");

	harness.start_server();
	let (attached, mut attached_client) =
		GuiRuntimeSession::connect_or_start(options).expect("launcher should attach to existing runtime");
	assert!(!attached.owns_server());
	let _ = outcome(&mut attached_client, document_replace("attached"));
	assert_eq!(document_text(&mut harness.ipc_client()), "attached");
}

#[test]
fn gui_private_state_survives_reconnect_e2e() {
	let harness = Harness::new();
	let options = gui_options(&harness);
	let (mut owner, mut first) =
		GuiRuntimeSession::connect_or_start(options.clone()).expect("first GUI session should start runtime");
	assert!(owner.owns_server());

	first
		.execute_gui(GuiCommand::SidebarSelect(SidebarTab::Inspect))
		.expect("sidebar update should succeed");
	first
		.execute_gui(GuiCommand::ShowBaselinesSet(true))
		.expect("baseline toggle should succeed");
	first
		.execute_gui(GuiCommand::ViewportScrollTo { x: 0.0, y: 96.0 })
		.expect("scroll update should succeed");
	let first_frame = first.gui_frame().expect("first GUI frame should load");
	assert_eq!(first_frame.ui.active_tab, SidebarTab::Inspect);
	assert!(first_frame.ui.show_baselines);
	assert!((first_frame.ui.canvas_scroll_y - 96.0).abs() <= f32::EPSILON);

	let (attached, mut second) =
		GuiRuntimeSession::connect_or_start(options).expect("second GUI session should attach");
	assert!(!attached.owns_server());
	let second_frame = second.gui_frame().expect("attached GUI frame should load");
	assert_eq!(second_frame.ui.active_tab, SidebarTab::Inspect);
	assert!(second_frame.ui.show_baselines);
	assert!((second_frame.ui.canvas_scroll_y - 96.0).abs() <= f32::EPSILON);

	second
		.execute_gui(GuiCommand::ShowHitboxesSet(true))
		.expect("attached GUI private update should succeed");
	let updated_frame = first.gui_frame().expect("owner GUI frame should refresh");
	assert!(updated_frame.ui.show_hitboxes);

	owner.shutdown().expect("owner shutdown should succeed");
}

#[test]
fn editor_command_to_document_text_e2e() {
	let mut host = Harness::new().runtime();
	let _ = outcome(&mut host, document_replace("abc"));
	let _ = outcome(&mut host, editor_mode(EditorModeCommand::EnterInsertAfter));
	let _ = outcome(&mut host, editor_motion(EditorMotion::LineEnd));
	let _ = outcome(&mut host, editor_insert("!"));

	assert_eq!(document_text(&mut host), "abc!");
	let editor = editor_state(&mut host);
	assert!(editor.undo_depth > 0);
	assert_eq!(editor.mode, EditorMode::Insert);

	let _ = outcome(&mut host, editor_history(EditorHistoryCommand::Undo));
	assert_eq!(document_text(&mut host), "abc");
}

#[test]
fn revision_monotonicity_test() {
	let mut host = Harness::new().runtime();
	let initial = editor_state(&mut host).revisions;

	let config = outcome(
		&mut host,
		config_set("editor.wrapping", GlorpValue::String("glyph".into())),
	);
	assert!(config.revisions.config > initial.config);
	assert!(config.revisions.editor >= initial.editor);

	let editor = outcome(&mut host, document_replace("abc"));
	assert!(editor.revisions.editor > config.revisions.editor);
	assert_eq!(editor.revisions.config, config.revisions.config);
}

#[test]
fn ipc_client_parity_test() {
	let direct = run_standard_transcript(&mut Harness::new().runtime());

	let mut ipc_harness = Harness::new();
	ipc_harness.start_server();
	let ipc = run_standard_transcript(&mut ipc_harness.ipc_client());

	let mut plugin_test = PluginTest::new("glorp", GlorpPlugin.into()).expect("plugin test should build");
	let mut plugin_harness = Harness::new();
	plugin_harness.start_server();
	let _ = eval_to_value(
		&mut plugin_test,
		&format!(
			r#"glorp call config-set {{path: "editor.wrapping", value: "word"}} --socket "{}""#,
			plugin_harness.socket_path.display()
		),
	);
	let _ = eval_to_value(
		&mut plugin_test,
		&format!(
			r#"glorp call document-replace {{text: "hello"}} --socket "{}""#,
			plugin_harness.socket_path.display()
		),
	);
	let _ = eval_to_value(
		&mut plugin_test,
		&format!(
			r#"glorp call editor-mode {{mode: "enter-insert-after"}} --socket "{}""#,
			plugin_harness.socket_path.display()
		),
	);
	let _ = eval_to_value(
		&mut plugin_test,
		&format!(
			r#"glorp call editor-motion {{motion: "line-end"}} --socket "{}""#,
			plugin_harness.socket_path.display()
		),
	);
	let _ = eval_to_value(
		&mut plugin_test,
		&format!(
			r#"glorp call editor-insert {{text: " world"}} --socket "{}""#,
			plugin_harness.socket_path.display()
		),
	);

	let plugin = host_state(&mut plugin_harness.ipc_client());
	assert_eq!(ipc.0, direct.0);
	assert_eq!(plugin.0, direct.0);
	assert_eq!(ipc.1.mode, direct.1.mode);
	assert_eq!(plugin.1.mode, direct.1.mode);
	assert_eq!(ipc.2.editor.wrapping, direct.2.editor.wrapping);
	assert_eq!(plugin.2.editor.wrapping, direct.2.editor.wrapping);
}

#[test]
fn plugin_auto_starts_shared_host_e2e() {
	let harness = Harness::new();
	set_host_bin();
	let mut plugin_test = PluginTest::new("glorp", GlorpPlugin.into()).expect("plugin test should build");
	let _ = eval_to_value(
		&mut plugin_test,
		&format!(
			r#"glorp call document-replace {{text: "shared-socket"}} --repo-root "{}""#,
			harness.root.display()
		),
	);
	assert_eq!(document_text(&mut harness.ipc_client()), "shared-socket");
	let _ = eval_to_value(
		&mut plugin_test,
		&format!(
			r#"glorp call session-shutdown --repo-root "{}""#,
			harness.root.display()
		),
	);
}

#[test]
fn persistence_smoke_test() {
	let harness = Harness::new();
	let paths = harness.paths.clone();
	let mut host = harness.runtime();
	let _ = outcome(
		&mut host,
		config_set("editor.wrapping", GlorpValue::String("glyph".to_owned())),
	);
	let _ = calls::ConfigPersist::call(&mut host, ()).expect("config-persist should succeed");
	drop(host);

	let mut fresh = RuntimeHost::new(RuntimeOptions { paths }).expect("fresh runtime should start");
	let config = config(&mut fresh);
	assert_eq!(config.editor.wrapping, WrapChoice::Glyph);
	assert!(
		!std::fs::read_to_string(harness.paths.durable_config_path.clone())
			.expect("config file")
			.contains("inspect:")
	);
}

#[test]
fn event_stream_conformance_test() {
	let mut host = Harness::new().runtime();
	let token = subscribe_changes(&mut host);

	let _ = outcome(
		&mut host,
		config_set("editor.wrapping", GlorpValue::String("glyph".to_owned())),
	);
	let _ = outcome(&mut host, document_replace("event stream"));
	assert!(
		host.call(config_set("editor.wrapping", GlorpValue::String("invalid".to_owned())))
			.is_err()
	);

	let first = match next_event(&mut host, token) {
		Some(GlorpEvent::Changed(event)) => event,
		other => panic!("unexpected first event: {other:?}"),
	};
	let second = match next_event(&mut host, token) {
		Some(GlorpEvent::Changed(event)) => event,
		other => panic!("unexpected second event: {other:?}"),
	};

	assert!(next_event(&mut host, token).is_none());
	assert!(first.delta.config_changed);
	assert_eq!(first.changed_config_paths, vec!["editor.wrapping".to_owned()]);
	assert!(second.delta.text_changed);
	assert!(second.revisions.editor >= first.revisions.editor);
}

#[test]
fn plugin_transcript_smoke_test() {
	let harness = Harness::new();
	set_host_bin();
	let mut plugin_test = PluginTest::new("glorp", GlorpPlugin.into()).expect("plugin test should build");
	let repo_root = harness.root.display();

	let _ = eval_to_value(
		&mut plugin_test,
		&format!(r#"glorp call config-set {{path: "editor.wrapping", value: "word"}} --repo-root "{repo_root}""#),
	);
	let _ = eval_to_value(
		&mut plugin_test,
		&format!(r#"glorp call document-replace {{text: "hello"}} --repo-root "{repo_root}""#),
	);
	let _ = eval_to_value(
		&mut plugin_test,
		&format!(r#"glorp call editor-mode {{mode: "enter-insert-after"}} --repo-root "{repo_root}""#),
	);
	let _ = eval_to_value(
		&mut plugin_test,
		&format!(r#"glorp call editor-motion {{motion: "line-end"}} --repo-root "{repo_root}""#),
	);
	let _ = eval_to_value(
		&mut plugin_test,
		&format!(r#"glorp call editor-insert {{text: " world"}} --repo-root "{repo_root}""#),
	);

	let text = eval_to_value(
		&mut plugin_test,
		&format!(r#"glorp call document-text --repo-root "{repo_root}""#),
	);
	let editor = eval_to_value(
		&mut plugin_test,
		&format!(r#"glorp call editor --repo-root "{repo_root}""#),
	);

	assert_eq!(text.coerce_str().expect("text should be string"), "hello world");
	assert_eq!(string_field(&editor, "mode"), "insert");
	assert!(int_field(&editor, "undo_depth") > 0);

	let _ = eval_to_value(
		&mut plugin_test,
		&format!(r#"glorp call session-shutdown --repo-root "{repo_root}""#),
	);
}

#[test]
fn plugin_transaction_e2e() {
	let harness = Harness::new();
	set_host_bin();
	let mut plugin_test = PluginTest::new("glorp", GlorpPlugin.into()).expect("plugin test should build");
	let repo_root = harness.root.display();

	let _ = eval_to_value(
		&mut plugin_test,
		&format!(
			r#"
			glorp call txn {{
			  calls: [
			    {{id: "config-set", input: {{path: "editor.wrapping", value: "glyph"}}}}
			    {{id: "document-replace", input: {{text: "hello"}}}}
			    {{id: "editor-mode", input: {{mode: "enter-insert-after"}}}}
			    {{id: "editor-motion", input: {{motion: "line-end"}}}}
			    {{id: "editor-insert", input: {{text: " world"}}}}
			  ]
			}} --repo-root "{repo_root}"
			"#,
		),
	);

	assert_eq!(document_text(&mut harness.ipc_client()), "hello world");
	let editor = editor_state(&mut harness.ipc_client());
	assert_eq!(editor.mode, EditorMode::Insert);
}

#[test]
fn plugin_session_attach_and_event_polling_e2e() {
	let mut harness = Harness::new();
	harness.start_server();
	let mut plugin_test = PluginTest::new("glorp", GlorpPlugin.into()).expect("plugin test should build");

	let session = eval_to_value(
		&mut plugin_test,
		&format!(r#"glorp call session-attach --repo-root "{}""#, harness.root.display()),
	);
	let socket_value = session.get_data_by_key("socket").expect("socket field");
	let socket = socket_value.coerce_str().expect("socket should be string");
	assert_eq!(socket, harness.socket_path.display().to_string());

	let stream = eval_to_value(
		&mut plugin_test,
		&format!(
			r#"glorp call events-subscribe --repo-root "{}""#,
			harness.root.display()
		),
	);
	let token = int_field(&stream, "token");
	let _ = eval_to_value(
		&mut plugin_test,
		&format!(
			r#"glorp call document-replace {{text: "eventful"}} --repo-root "{}""#,
			harness.root.display()
		),
	);
	let event = eval_to_value(
		&mut plugin_test,
		&format!(
			r#"glorp call events-next {{token: {}}} --repo-root "{}""#,
			token,
			harness.root.display(),
		),
	);
	let kind = string_field(&event, "kind");
	assert_eq!(kind, "changed");

	let _ = eval_to_value(
		&mut plugin_test,
		&format!(
			r#"glorp call events-unsubscribe {{token: {}}} --repo-root "{}""#,
			token,
			harness.root.display(),
		),
	);
}

#[test]
fn generated_surface_artifact_golden_test() {
	let repo_root = repo_root();
	assert_eq!(
		std::fs::read_to_string(repo_root.join("schema/glorp-schema.json")).expect("schema file"),
		serde_json::to_string_pretty(&glorp_api::glorp_schema()).expect("schema json"),
	);
	assert_eq!(
		std::fs::read_to_string(repo_root.join("nu/glorp.nu")).expect("Nu module"),
		glorp_api::render_nu_module(),
	);
	assert_eq!(
		std::fs::read_to_string(repo_root.join("nu/completions.nu")).expect("Nu completions"),
		glorp_api::render_nu_completions(),
	);
}
