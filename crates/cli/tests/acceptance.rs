use {
	glorp_api::*,
	glorp_gui::{GlorpGui, GuiLaunchOptions, GuiMessage, GuiRuntimeSession},
	glorp_nu_plugin::GlorpPlugin,
	glorp_runtime::{ConfigStore, ConfigStorePaths, RuntimeHost, RuntimeOptions, export_surface_artifacts},
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

fn snapshot(host: &mut impl GlorpHost, scene: SceneLevel) -> GlorpSnapshot {
	match host
		.query(GlorpQuery::Snapshot(SnapshotQuery {
			scene,
			include_document_text: true,
		}))
		.expect("snapshot query should succeed")
	{
		GlorpQueryResult::Snapshot(snapshot) => snapshot,
		other => panic!("unexpected snapshot response: {other:?}"),
	}
}

fn document_text(host: &mut impl GlorpHost) -> String {
	match host
		.query(GlorpQuery::DocumentText)
		.expect("document text query should succeed")
	{
		GlorpQueryResult::DocumentText(text) => text,
		other => panic!("unexpected document text response: {other:?}"),
	}
}

fn assert_f32_eq(actual: f32, expected: f32) {
	assert!((actual - expected).abs() <= f32::EPSILON);
}

fn txn(execs: Vec<GlorpExec>) -> GlorpExec {
	GlorpExec::Txn(GlorpTxn { execs })
}

fn config_set(path: &str, value: GlorpValue) -> GlorpExec {
	GlorpExec::ConfigSet(ConfigAssignment {
		path: path.to_owned(),
		value,
	})
}

fn config_persist() -> GlorpExec {
	GlorpExec::ConfigPersist
}

fn document_replace(text: &str) -> GlorpExec {
	GlorpExec::DocumentReplace(TextInput { text: text.to_owned() })
}

fn editor_mode(mode: EditorModeCommand) -> GlorpExec {
	GlorpExec::EditorMode(EditorModeInput { mode })
}

fn editor_motion(motion: EditorMotion) -> GlorpExec {
	GlorpExec::EditorMotion(EditorMotionInput { motion })
}

fn editor_insert(text: &str) -> GlorpExec {
	GlorpExec::EditorInsert(TextInput { text: text.to_owned() })
}

fn editor_history(action: EditorHistoryCommand) -> GlorpExec {
	GlorpExec::EditorHistory(EditorHistoryInput { action })
}

fn scene_ensure() -> GlorpExec {
	GlorpExec::SceneEnsure
}

fn ui_sidebar_select(tab: SidebarTab) -> GlorpExec {
	GlorpExec::UiSidebarSelect(SidebarTabInput { tab })
}

fn ui_inspect_target_select(target: Option<CanvasTarget>) -> GlorpExec {
	GlorpExec::UiInspectTargetSelect(InspectTargetInput { target })
}

fn run_standard_transcript(host: &mut impl GlorpHost) -> GlorpSnapshot {
	host.execute(config_set("editor.wrapping", GlorpValue::String("word".to_owned())))
		.expect("config set should succeed");
	host.execute(document_replace("hello"))
		.expect("document replace should succeed");
	host.execute(editor_mode(EditorModeCommand::EnterInsertAfter))
		.expect("enter insert should succeed");
	host.execute(editor_motion(EditorMotion::LineEnd))
		.expect("line-end should succeed");
	host.execute(editor_insert(" world")).expect("insert should succeed");
	host.execute(scene_ensure()).expect("scene ensure should succeed");
	snapshot(host, SceneLevel::Materialize)
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

#[test]
fn schema_export_smoke_test() {
	let harness = Harness::new();
	harness.export_surface();
	let mut host = harness.runtime();

	let schema = match host.query(GlorpQuery::Schema).expect("schema query should succeed") {
		GlorpQueryResult::Schema(schema) => schema,
		other => panic!("unexpected schema response: {other:?}"),
	};

	assert_eq!(schema.version, 3);
	assert!(
		schema
			.operations
			.iter()
			.any(|operation| { operation.kind == OperationKind::Exec && operation.id == "config-set" })
	);
	assert!(
		schema
			.operations
			.iter()
			.any(|operation| { operation.kind == OperationKind::Exec && operation.id == "editor-motion" })
	);
	assert!(
		schema
			.operations
			.iter()
			.any(|operation| { operation.kind == OperationKind::Exec && operation.id == "scene-ensure" })
	);
	assert!(
		schema
			.operations
			.iter()
			.any(|operation| { operation.kind == OperationKind::Helper && operation.id == "session-attach" })
	);
	assert!(
		schema
			.operations
			.iter()
			.any(|operation| { operation.kind == OperationKind::Helper && operation.id == "events-subscribe" })
	);
	assert!(schema.types.iter().any(|ty| ty.name == "GlorpConfig"));
	assert!(schema.types.iter().any(|ty| ty.name == "WrapChoice"));
	assert!(schema.types.iter().any(|ty| ty.name == "InspectConfig"));
	assert!(schema.types.iter().all(|ty| !ty.docs.is_empty()));
	assert!(harness.paths.schema_path.exists());
}

#[test]
fn nu_plugin_roundtrip_smoke_test() {
	let mut harness = Harness::new();
	harness.start_server();

	let mut plugin_test = PluginTest::new("glorp", GlorpPlugin.into()).expect("plugin test should build");
	let before = eval_to_value(
		&mut plugin_test,
		&format!(r#"glorp query config --socket "{}""#, harness.socket_path.display()),
	);
	let _ = eval_to_value(
		&mut plugin_test,
		&format!(
			r#"glorp exec config-set {{path: "editor.wrapping", value: "glyph"}} --socket "{}""#,
			harness.socket_path.display()
		),
	);
	let after = eval_to_value(
		&mut plugin_test,
		&format!(r#"glorp query config --socket "{}""#, harness.socket_path.display()),
	);

	assert_ne!(before, after);
	let wrapping_value = after
		.get_data_by_key("editor")
		.expect("editor field")
		.get_data_by_key("wrapping")
		.expect("wrapping field");
	let after_wrapping = wrapping_value.coerce_str().expect("wrapping should be string");
	assert_eq!(after_wrapping, "glyph");

	let snapshot = snapshot(&mut harness.ipc_client(), SceneLevel::IfReady);
	assert_eq!(snapshot.revisions.config, 2);
}

#[test]
fn invalid_config_rejection_e2e() {
	let mut host = Harness::new().runtime();
	let before = serde_json::to_vec(&snapshot(&mut host, SceneLevel::IfReady)).expect("serialize snapshot");
	let config_before = snapshot(&mut host, SceneLevel::IfReady).revisions.config;

	let error = host
		.execute(config_set(
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

	let after_snapshot = snapshot(&mut host, SceneLevel::IfReady);
	assert_eq!(config_before, after_snapshot.revisions.config);
	assert_eq!(before, serde_json::to_vec(&after_snapshot).expect("serialize snapshot"));
}

#[test]
fn transaction_atomicity_e2e() {
	let mut host = Harness::new().runtime();
	let token = host
		.subscribe(GlorpSubscription::Changes)
		.expect("subscribe should succeed");
	let before = snapshot(&mut host, SceneLevel::IfReady);

	let error = host
		.execute(txn(vec![
			config_set("editor.wrapping", GlorpValue::String("glyph".into())),
			document_replace("changed"),
			config_set("editor.wrapping", GlorpValue::String("invalid-value".into())),
		]))
		.expect_err("transaction should fail");
	assert!(matches!(error, GlorpError::Validation { .. }));

	let after = snapshot(&mut host, SceneLevel::IfReady);
	assert_eq!(before.document_text, after.document_text);
	assert_eq!(before.config, after.config);
	assert_eq!(before.revisions, after.revisions);
	assert!(host.next_event(token).expect("next event should succeed").is_none());
}

#[test]
fn nested_transaction_rejection_e2e() {
	let mut host = Harness::new().runtime();
	let error = host
		.execute(txn(vec![txn(vec![scene_ensure()])]))
		.expect_err("nested transaction should fail");

	match error {
		GlorpError::Validation { message, .. } => assert!(message.contains("nested transactions")),
		other => panic!("unexpected error: {other:?}"),
	}
}

#[test]
fn gui_runtime_snapshot_e2e() {
	let mut harness = Harness::new();
	harness.start_server();

	let mut gui = GlorpGui::new(harness.ipc_client());
	gui.send(GuiMessage::SidebarSelect(SidebarTab::Inspect))
		.expect("sidebar select should succeed");
	gui.send(GuiMessage::ViewportScrollTo { x: 0.0, y: 120.0 })
		.expect("scroll should succeed");

	let mut client = harness.ipc_client();
	let gui_snapshot = snapshot(&mut client, SceneLevel::IfReady);
	assert_eq!(gui_snapshot.ui.active_tab, SidebarTab::Inspect);
	assert_f32_eq(gui_snapshot.ui.canvas_scroll_y, 120.0);

	drop(gui);
	let reconnect = snapshot(&mut harness.ipc_client(), SceneLevel::IfReady);
	assert_eq!(reconnect.ui.active_tab, SidebarTab::Inspect);
	assert_f32_eq(reconnect.ui.canvas_scroll_y, 120.0);
}

#[test]
fn gui_launcher_socket_contract_e2e() {
	let mut harness = Harness::new();
	let options = GuiLaunchOptions {
		repo_root: harness.root.clone(),
		socket_path: default_socket_path(&harness.root),
	};
	let (mut launched, mut launched_client) =
		GuiRuntimeSession::connect_or_start(options.clone()).expect("launcher should start runtime");
	assert!(launched.owns_server());
	launched_client
		.execute(document_replace("launched"))
		.expect("launched client should write");
	assert_eq!(document_text(&mut launched_client), "launched");
	launched.shutdown().expect("launcher shutdown should succeed");

	harness.start_server();
	let (attached, mut attached_client) =
		GuiRuntimeSession::connect_or_start(options).expect("launcher should attach to existing runtime");
	assert!(!attached.owns_server());
	attached_client
		.execute(document_replace("attached"))
		.expect("attached client should write");
	assert_eq!(document_text(&mut harness.ipc_client()), "attached");
}

#[test]
fn editor_command_to_document_text_e2e() {
	let mut host = Harness::new().runtime();
	host.execute(document_replace("abc")).expect("replace should succeed");
	host.execute(editor_mode(EditorModeCommand::EnterInsertAfter))
		.expect("enter insert should succeed");
	host.execute(editor_motion(EditorMotion::LineEnd))
		.expect("move to line end should succeed");
	host.execute(editor_insert("!")).expect("insert should succeed");

	assert_eq!(document_text(&mut host), "abc!");
	let snapshot = snapshot(&mut host, SceneLevel::IfReady);
	assert!(snapshot.editor.undo_depth > 0);

	host.execute(editor_history(EditorHistoryCommand::Undo))
		.expect("undo should succeed");
	assert_eq!(document_text(&mut host), "abc");
}

#[test]
fn scene_materialization_proof_test() {
	let mut host = Harness::new().runtime();
	host.execute(document_replace("fixture text\nwith two lines"))
		.expect("replace should succeed");

	let omitted = snapshot(&mut host, SceneLevel::Omit);
	assert!(omitted.scene.is_none());

	let materialized = snapshot(&mut host, SceneLevel::Materialize);
	let scene = materialized.scene.expect("scene should be present");
	assert!(scene.measured_width > 0.0);
	assert!(scene.measured_height > 0.0);
	assert!(scene.run_count > 0);
	assert!(scene.cluster_count > 0);
	let stable = snapshot(&mut host, SceneLevel::Materialize);
	assert_eq!(stable.scene.expect("scene should remain").revision, scene.revision);
}

#[test]
fn revision_monotonicity_test() {
	let mut host = Harness::new().runtime();
	let initial = snapshot(&mut host, SceneLevel::IfReady).revisions;

	let config = host
		.execute(config_set("inspect.show_hitboxes", GlorpValue::Bool(true)))
		.expect("config update should succeed");
	assert!(config.revisions.config > initial.config);
	assert_eq!(config.revisions.editor, initial.editor);

	let editor = host
		.execute(document_replace("abc"))
		.expect("document replace should succeed");
	assert!(editor.revisions.editor > config.revisions.editor);
	assert_eq!(editor.revisions.config, config.revisions.config);

	let scene = host.execute(scene_ensure()).expect("scene ensure should succeed");
	assert!(scene.revisions.scene.is_some());

	let repeated = host
		.execute(scene_ensure())
		.expect("scene ensure repeat should succeed");
	assert_eq!(repeated.revisions, scene.revisions);
}

#[test]
fn ipc_client_parity_test() {
	let direct = run_standard_transcript(&mut Harness::new().runtime());

	let mut harness = Harness::new();
	harness.start_server();
	let ipc = run_standard_transcript(&mut harness.ipc_client());

	let mut plugin_test = PluginTest::new("glorp", GlorpPlugin.into()).expect("plugin test should build");
	let _ = eval_to_value(
		&mut plugin_test,
		&format!(
			r#"glorp exec config-set {{path: "editor.wrapping", value: "word"}} --socket "{}""#,
			harness.socket_path.display()
		),
	);
	let _ = eval_to_value(
		&mut plugin_test,
		&format!(
			r#"glorp exec document-replace {{text: "hello"}} --socket "{}""#,
			harness.socket_path.display()
		),
	);
	let _ = eval_to_value(
		&mut plugin_test,
		&format!(
			r#"glorp exec editor-mode {{mode: "enter-insert-after"}} --socket "{}""#,
			harness.socket_path.display()
		),
	);
	let _ = eval_to_value(
		&mut plugin_test,
		&format!(
			r#"glorp exec editor-motion {{motion: "line-end"}} --socket "{}""#,
			harness.socket_path.display()
		),
	);
	let _ = eval_to_value(
		&mut plugin_test,
		&format!(
			r#"glorp exec editor-insert {{text: " world"}} --socket "{}""#,
			harness.socket_path.display()
		),
	);
	let _ = eval_to_value(
		&mut plugin_test,
		&format!(
			r#"glorp exec scene-ensure --socket "{}""#,
			harness.socket_path.display()
		),
	);
	let plugin_snapshot = snapshot(&mut harness.ipc_client(), SceneLevel::Materialize);

	assert_eq!(ipc.document_text, direct.document_text);
	assert_eq!(plugin_snapshot.document_text, direct.document_text);
	assert_eq!(ipc.config.editor.wrapping, direct.config.editor.wrapping);
	assert_eq!(plugin_snapshot.config.editor.wrapping, direct.config.editor.wrapping);
}

#[test]
fn plugin_auto_starts_shared_host_e2e() {
	let harness = Harness::new();
	unsafe {
		std::env::set_var("GLORP_HOST_BIN", host_bin());
	}
	let mut plugin_test = PluginTest::new("glorp", GlorpPlugin.into()).expect("plugin test should build");
	let _ = eval_to_value(
		&mut plugin_test,
		&format!(
			r#"glorp exec document-replace {{text: "shared-socket"}} --repo-root "{}""#,
			harness.root.display()
		),
	);
	assert_eq!(document_text(&mut harness.ipc_client()), "shared-socket");
	let _ = eval_to_value(
		&mut plugin_test,
		&format!(
			r#"glorp helper session-shutdown --repo-root "{}""#,
			harness.root.display()
		),
	);
}

#[test]
fn persistence_smoke_test() {
	let harness = Harness::new();
	let paths = harness.paths.clone();
	let mut host = harness.runtime();
	host.execute(config_set("editor.wrapping", GlorpValue::String("glyph".to_owned())))
		.expect("config set should succeed");
	host.execute(config_persist()).expect("persist should succeed");
	drop(host);

	let mut fresh = RuntimeHost::new(RuntimeOptions { paths }).expect("fresh runtime should start");
	let config = match fresh.query(GlorpQuery::Config).expect("config query should succeed") {
		GlorpQueryResult::Config(config) => config,
		other => panic!("unexpected config response: {other:?}"),
	};
	assert_eq!(config.editor.wrapping, WrapChoice::Glyph);
}

#[test]
fn event_stream_conformance_test() {
	let mut host = Harness::new().runtime();
	let token = host
		.subscribe(GlorpSubscription::Changes)
		.expect("subscribe should succeed");

	host.execute(config_set("inspect.show_hitboxes", GlorpValue::Bool(true)))
		.expect("config update should succeed");
	host.execute(document_replace("event stream"))
		.expect("document replace should succeed");
	host.execute(scene_ensure()).expect("scene ensure should succeed");
	assert!(
		host.execute(config_set("editor.wrapping", GlorpValue::String("invalid".to_owned()),))
			.is_err()
	);

	let first = match host.next_event(token).expect("event should be available") {
		Some(GlorpEvent::Changed(event)) => event,
		other => panic!("unexpected first event: {other:?}"),
	};
	let second = match host.next_event(token).expect("event should be available") {
		Some(GlorpEvent::Changed(event)) => event,
		other => panic!("unexpected second event: {other:?}"),
	};
	let third = match host.next_event(token).expect("event should be available") {
		Some(GlorpEvent::Changed(event)) => event,
		other => panic!("unexpected third event: {other:?}"),
	};

	assert!(host.next_event(token).expect("stream read should succeed").is_none());
	assert!(first.revisions.config < second.revisions.config || first.revisions.editor < second.revisions.editor);
	assert!(second.revisions.scene <= third.revisions.scene);
	assert_eq!(first.changed_config_paths, vec!["inspect.show_hitboxes".to_owned()]);
	assert!(first.delta.config_changed);
	assert!(second.delta.text_changed);
	assert!(third.delta.scene_changed);
}

#[test]
fn plugin_transcript_smoke_test() {
	let harness = Harness::new();
	unsafe {
		std::env::set_var("GLORP_HOST_BIN", host_bin());
	}
	let mut plugin_test = PluginTest::new("glorp", GlorpPlugin.into()).expect("plugin test should build");
	let repo_root = harness.root.display();

	let _ = eval_to_value(
		&mut plugin_test,
		&format!(r#"glorp exec config-set {{path: "editor.wrapping", value: "word"}} --repo-root "{repo_root}""#),
	);
	let _ = eval_to_value(
		&mut plugin_test,
		&format!(r#"glorp exec document-replace {{text: "hello"}} --repo-root "{repo_root}""#),
	);
	let _ = eval_to_value(
		&mut plugin_test,
		&format!(r#"glorp exec editor-mode {{mode: "enter-insert-after"}} --repo-root "{repo_root}""#),
	);
	let _ = eval_to_value(
		&mut plugin_test,
		&format!(r#"glorp exec editor-motion {{motion: "line-end"}} --repo-root "{repo_root}""#),
	);
	let _ = eval_to_value(
		&mut plugin_test,
		&format!(r#"glorp exec editor-insert {{text: " world"}} --repo-root "{repo_root}""#),
	);
	let _ = eval_to_value(
		&mut plugin_test,
		&format!(r#"glorp exec scene-ensure --repo-root "{repo_root}""#),
	);

	let snapshot = snapshot(&mut harness.ipc_client(), SceneLevel::Materialize);

	assert_eq!(snapshot.config.editor.wrapping, WrapChoice::Word);
	assert_eq!(snapshot.document_text.as_deref(), Some("hello world"));
	assert!(snapshot.revisions.editor > 0);
	assert!(snapshot.revisions.scene.is_some());
	assert!(snapshot.editor.undo_depth > 0);
	assert_eq!(snapshot.ui.active_tab, SidebarTab::Controls);
	assert_f32_eq(snapshot.ui.canvas_scroll_y, 0.0);

	let _ = eval_to_value(
		&mut plugin_test,
		&format!(r#"glorp helper session-shutdown --repo-root "{repo_root}""#),
	);
}

#[test]
fn plugin_transaction_e2e() {
	let harness = Harness::new();
	unsafe {
		std::env::set_var("GLORP_HOST_BIN", host_bin());
	}
	let mut plugin_test = PluginTest::new("glorp", GlorpPlugin.into()).expect("plugin test should build");
	let repo_root = harness.root.display();

	let _ = eval_to_value(
		&mut plugin_test,
		&format!(
			r#"
			glorp exec txn {{
			  execs: [
			    {{op: "config-set", input: {{path: "editor.wrapping", value: "glyph"}}}}
			    {{op: "document-replace", input: {{text: "hello"}}}}
			    {{op: "editor-mode", input: {{mode: "enter-insert-after"}}}}
			    {{op: "editor-motion", input: {{motion: "line-end"}}}}
			    {{op: "editor-insert", input: {{text: " world"}}}}
			    {{op: "scene-ensure"}}
			  ]
			}} --repo-root "{repo_root}"
			"#,
		),
	);

	let snapshot = snapshot(&mut harness.ipc_client(), SceneLevel::Materialize);
	assert_eq!(snapshot.config.editor.wrapping, WrapChoice::Glyph);
	assert_eq!(snapshot.document_text.as_deref(), Some("hello world"));
}

#[test]
fn selection_query_e2e() {
	let mut host = Harness::new().runtime();
	host.execute(document_replace("alpha beta"))
		.expect("replace should succeed");

	let selection = match host
		.query(GlorpQuery::Selection)
		.expect("selection query should succeed")
	{
		GlorpQueryResult::Selection(selection) => selection,
		other => panic!("unexpected selection response: {other:?}"),
	};

	assert_eq!(selection.mode, EditorMode::Normal);
	assert!(selection.range.is_some());
	assert_eq!(selection.selected_text.as_deref(), Some("a"));
	assert!(selection.selection_head.is_some());
}

#[test]
fn inspect_details_query_e2e() {
	let mut host = Harness::new().runtime();
	host.execute(document_replace("inspect me"))
		.expect("replace should succeed");
	host.execute(ui_sidebar_select(SidebarTab::Inspect))
		.expect("sidebar select should succeed");
	host.execute(scene_ensure()).expect("scene ensure should succeed");
	host.execute(ui_inspect_target_select(Some(CanvasTarget::Cluster(0))))
		.expect("inspect target select should succeed");

	let inspect = match host
		.query(GlorpQuery::InspectDetails(InspectDetailsQuery { target: None }))
		.expect("inspect-details query should succeed")
	{
		GlorpQueryResult::InspectDetails(inspect) => inspect,
		other => panic!("unexpected inspect-details response: {other:?}"),
	};

	assert_eq!(inspect.active_target, Some(CanvasTarget::Cluster(0)));
	assert!(inspect.scene.is_some());
	assert!(inspect.interaction_details.contains("cluster index: 0"));
}

#[test]
fn perf_dashboard_query_e2e() {
	let mut host = Harness::new().runtime();
	host.execute(document_replace("one\ntwo\nthree"))
		.expect("replace should succeed");

	let perf = match host
		.query(GlorpQuery::Perf)
		.expect("perf dashboard query should succeed")
	{
		GlorpQueryResult::Perf(perf) => perf,
		other => panic!("unexpected perf dashboard response: {other:?}"),
	};

	assert!(perf.overview.scene_ready);
	assert!(perf.overview.scene_revision.is_some());
	assert!(!perf.metrics.is_empty());
	assert_eq!(perf.metrics[0].label, "scene.build");
	assert!(perf.metrics[0].total_samples >= 1);
}

#[test]
fn plugin_session_attach_and_event_polling_e2e() {
	let mut harness = Harness::new();
	harness.start_server();
	let mut plugin_test = PluginTest::new("glorp", GlorpPlugin.into()).expect("plugin test should build");

	let session = eval_to_value(
		&mut plugin_test,
		&format!(
			r#"glorp helper session-attach --repo-root "{}""#,
			harness.root.display()
		),
	);
	let socket_value = session.get_data_by_key("socket").expect("socket field");
	let socket = socket_value.coerce_str().expect("socket should be string");
	assert_eq!(socket, harness.socket_path.display().to_string());

	let stream = eval_to_value(
		&mut plugin_test,
		&format!(
			r#"glorp helper events-subscribe --repo-root "{}""#,
			harness.root.display()
		),
	);
	let token = stream
		.get_data_by_key("token")
		.and_then(|value| match value {
			Value::Int { val, .. } => Some(val),
			_ => None,
		})
		.expect("token should be int");
	let _ = eval_to_value(
		&mut plugin_test,
		&format!(
			r#"glorp exec document-replace {{text: "eventful"}} --repo-root "{}""#,
			harness.root.display()
		),
	);
	let event = eval_to_value(
		&mut plugin_test,
		&format!(
			r#"glorp helper events-next {{token: {}}} --repo-root "{}""#,
			token,
			harness.root.display(),
		),
	);
	let kind = event
		.get_data_by_key("kind")
		.and_then(|value| value.coerce_str().ok().map(|value| value.into_owned()))
		.expect("event kind should be string");
	assert_eq!(kind, "changed");

	let _ = eval_to_value(
		&mut plugin_test,
		&format!(
			r#"glorp helper events-unsubscribe {{token: {}}} --repo-root "{}""#,
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
