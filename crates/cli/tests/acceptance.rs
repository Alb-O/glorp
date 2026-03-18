use {
	glorp_api::*,
	glorp_gui::{GlorpGui, GuiLaunchOptions, GuiMessage, GuiRuntimeSession},
	glorp_nu_plugin::GlorpPlugin,
	glorp_runtime::{ConfigStorePaths, RuntimeHost, RuntimeOptions},
	glorp_transport::{IpcClient, IpcServerHandle, default_socket_path, start_server},
	nu_plugin_test_support::PluginTest,
	nu_protocol::{Span, Value},
	serde_json::Value as JsonValue,
	std::{
		env,
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
		.query(GlorpQuery::Snapshot {
			scene,
			include_document_text: true,
		})
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

fn repo_root() -> PathBuf {
	PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.parent()
		.expect("cli crate should have a parent")
		.parent()
		.expect("repo root should exist")
		.to_path_buf()
}

fn nu_command() -> Command {
	let mut command = Command::new("nu");
	let cli_bin = PathBuf::from(env!("CARGO_BIN_EXE_glorp_cli"));
	let cli_dir = cli_bin.parent().expect("glorp_cli binary should have a parent");
	let existing_path = env::var("PATH").unwrap_or_default();
	let path = if existing_path.is_empty() {
		cli_dir.display().to_string()
	} else {
		format!("{}:{existing_path}", cli_dir.display())
	};
	command.env("PATH", path);
	command
}

fn run_standard_transcript(host: &mut impl GlorpHost) -> GlorpSnapshot {
	host.execute(GlorpCommand::Config(ConfigCommand::Set {
		path: "editor.wrapping".to_owned(),
		value: GlorpValue::String("word".to_owned()),
	}))
	.expect("config set should succeed");
	host.execute(GlorpCommand::Document(DocumentCommand::Replace {
		text: "hello".to_owned(),
	}))
	.expect("document replace should succeed");
	host.execute(GlorpCommand::Editor(EditorCommand::Mode(
		EditorModeCommand::EnterInsertAfter,
	)))
	.expect("enter insert should succeed");
	host.execute(GlorpCommand::Editor(EditorCommand::Motion(EditorMotion::LineEnd)))
		.expect("line-end should succeed");
	host.execute(GlorpCommand::Editor(EditorCommand::Edit(EditorEditCommand::Insert {
		text: " world".to_owned(),
	})))
	.expect("insert should succeed");
	host.execute(GlorpCommand::Scene(SceneCommand::Ensure))
		.expect("scene ensure should succeed");
	snapshot(host, SceneLevel::Materialize)
}

fn cli_json(harness: &Harness, args: &[&str]) -> JsonValue {
	let output = Command::new(env!("CARGO_BIN_EXE_glorp_cli"))
		.arg("--socket")
		.arg(&harness.socket_path)
		.args(args)
		.output()
		.expect("CLI should run");
	assert!(
		output.status.success(),
		"CLI stderr: {}",
		String::from_utf8_lossy(&output.stderr)
	);
	serde_json::from_slice(&output.stdout).expect("CLI output should be valid JSON")
}

fn cli_json_repo_root(harness: &Harness, args: &[&str]) -> JsonValue {
	let output = Command::new(env!("CARGO_BIN_EXE_glorp_cli"))
		.current_dir(&harness.root)
		.env_remove("GLORP_SOCKET")
		.args(args)
		.output()
		.expect("CLI should run");
	assert!(
		output.status.success(),
		"CLI stderr: {}",
		String::from_utf8_lossy(&output.stderr)
	);
	serde_json::from_slice(&output.stdout).expect("CLI output should be valid JSON")
}

#[test]
fn schema_export_smoke_test() {
	let harness = Harness::new();
	let mut host = harness.runtime();

	let schema = match host.query(GlorpQuery::Schema).expect("schema query should succeed") {
		GlorpQueryResult::Schema(schema) => schema,
		other => panic!("unexpected schema response: {other:?}"),
	};

	assert!(schema.version > 0);
	assert!(schema.commands.iter().any(|command| command.path == "glorp config set"));
	assert!(
		schema
			.commands
			.iter()
			.any(|command| command.path == "glorp editor motion")
	);
	assert!(
		schema
			.commands
			.iter()
			.any(|command| command.path == "glorp scene ensure")
	);
	assert!(schema.config.iter().any(|field| field.path == "editor.font"));
	assert!(schema.config.iter().any(|field| field.path == "editor.wrapping"));
	assert!(schema.config.iter().any(|field| field.path == "inspect.show_hitboxes"));
	assert!(schema.config.iter().all(|field| !field.docs.is_empty()));
	assert!(harness.paths.schema_path.exists());
}

#[test]
fn nu_plugin_roundtrip_smoke_test() {
	let mut harness = Harness::new();
	harness.start_server();

	let mut plugin_test = PluginTest::new("glorp", GlorpPlugin.into()).expect("plugin test should build");
	let before = eval_to_value(
		&mut plugin_test,
		&format!(r#"glorp get config --socket "{}""#, harness.socket_path.display()),
	);
	let _ = eval_to_value(
		&mut plugin_test,
		&format!(
			r#"glorp config set editor.wrapping glyph --socket "{}""#,
			harness.socket_path.display()
		),
	);
	let after = eval_to_value(
		&mut plugin_test,
		&format!(r#"glorp get config --socket "{}""#, harness.socket_path.display()),
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
		.execute(GlorpCommand::Config(ConfigCommand::Set {
			path: "editor.wrapping".to_owned(),
			value: GlorpValue::String("definitely-not-valid".to_owned()),
		}))
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
		.execute(GlorpCommand::Txn(GlorpTxn {
			commands: vec![
				GlorpCommand::Config(ConfigCommand::Set {
					path: "editor.wrapping".to_owned(),
					value: GlorpValue::String("glyph".to_owned()),
				}),
				GlorpCommand::Document(DocumentCommand::Replace {
					text: "changed".to_owned(),
				}),
				GlorpCommand::Config(ConfigCommand::Set {
					path: "editor.wrapping".to_owned(),
					value: GlorpValue::String("invalid-value".to_owned()),
				}),
			],
		}))
		.expect_err("transaction should fail");
	assert!(matches!(error, GlorpError::Validation { .. }));

	let after = snapshot(&mut host, SceneLevel::IfReady);
	assert_eq!(before.document_text, after.document_text);
	assert_eq!(before.config, after.config);
	assert_eq!(before.revisions, after.revisions);
	assert!(host.next_event(token).expect("next event should succeed").is_none());
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
	assert_eq!(gui_snapshot.ui.canvas_scroll_y, 120.0);

	drop(gui);
	let reconnect = snapshot(&mut harness.ipc_client(), SceneLevel::IfReady);
	assert_eq!(reconnect.ui.active_tab, SidebarTab::Inspect);
	assert_eq!(reconnect.ui.canvas_scroll_y, 120.0);
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
		.execute(GlorpCommand::Document(DocumentCommand::Replace {
			text: "launched".to_owned(),
		}))
		.expect("launched client should write");
	assert_eq!(document_text(&mut launched_client), "launched");
	launched.shutdown().expect("launcher shutdown should succeed");

	harness.start_server();
	let (attached, mut attached_client) =
		GuiRuntimeSession::connect_or_start(options).expect("launcher should attach to existing runtime");
	assert!(!attached.owns_server());
	attached_client
		.execute(GlorpCommand::Document(DocumentCommand::Replace {
			text: "attached".to_owned(),
		}))
		.expect("attached client should write");
	assert_eq!(document_text(&mut harness.ipc_client()), "attached");
}

#[test]
fn editor_command_to_document_text_e2e() {
	let mut host = Harness::new().runtime();
	host.execute(GlorpCommand::Document(DocumentCommand::Replace {
		text: "abc".to_owned(),
	}))
	.expect("replace should succeed");
	host.execute(GlorpCommand::Editor(EditorCommand::Mode(
		EditorModeCommand::EnterInsertAfter,
	)))
	.expect("enter insert should succeed");
	host.execute(GlorpCommand::Editor(EditorCommand::Motion(EditorMotion::LineEnd)))
		.expect("move to line end should succeed");
	host.execute(GlorpCommand::Editor(EditorCommand::Edit(EditorEditCommand::Insert {
		text: "!".to_owned(),
	})))
	.expect("insert should succeed");

	assert_eq!(document_text(&mut host), "abc!");
	let snapshot = snapshot(&mut host, SceneLevel::IfReady);
	assert!(snapshot.editor.undo_depth > 0);

	host.execute(GlorpCommand::Editor(EditorCommand::History(EditorHistoryCommand::Undo)))
		.expect("undo should succeed");
	assert_eq!(document_text(&mut host), "abc");
}

#[test]
fn scene_materialization_proof_test() {
	let mut host = Harness::new().runtime();
	host.execute(GlorpCommand::Document(DocumentCommand::Replace {
		text: "fixture text\nwith two lines".to_owned(),
	}))
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
		.execute(GlorpCommand::Config(ConfigCommand::Set {
			path: "inspect.show_hitboxes".to_owned(),
			value: GlorpValue::Bool(true),
		}))
		.expect("config update should succeed");
	assert!(config.revisions.config > initial.config);
	assert_eq!(config.revisions.editor, initial.editor);

	let editor = host
		.execute(GlorpCommand::Document(DocumentCommand::Replace {
			text: "abc".to_owned(),
		}))
		.expect("document replace should succeed");
	assert!(editor.revisions.editor > config.revisions.editor);
	assert_eq!(editor.revisions.config, config.revisions.config);

	let scene = host
		.execute(GlorpCommand::Scene(SceneCommand::Ensure))
		.expect("scene ensure should succeed");
	assert!(scene.revisions.scene.is_some());

	let repeated = host
		.execute(GlorpCommand::Scene(SceneCommand::Ensure))
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
			r#"glorp config set editor.wrapping word --socket "{}""#,
			harness.socket_path.display()
		),
	);
	let _ = eval_to_value(
		&mut plugin_test,
		&format!(
			r#"glorp doc replace "hello" --socket "{}""#,
			harness.socket_path.display()
		),
	);
	let _ = eval_to_value(
		&mut plugin_test,
		&format!(
			r#"glorp editor mode enter-insert-after --socket "{}""#,
			harness.socket_path.display()
		),
	);
	let _ = eval_to_value(
		&mut plugin_test,
		&format!(
			r#"glorp editor motion line-end --socket "{}""#,
			harness.socket_path.display()
		),
	);
	let _ = eval_to_value(
		&mut plugin_test,
		&format!(
			r#"glorp editor edit insert " world" --socket "{}""#,
			harness.socket_path.display()
		),
	);
	let _ = eval_to_value(
		&mut plugin_test,
		&format!(r#"glorp scene ensure --socket "{}""#, harness.socket_path.display()),
	);
	let plugin_snapshot = snapshot(&mut harness.ipc_client(), SceneLevel::Materialize);

	let cli_harness = {
		let mut harness = Harness::new();
		harness.start_server();
		let _ = cli_json(&harness, &["config", "set", "editor.wrapping", "word"]);
		let _ = cli_json(&harness, &["doc", "replace", "hello"]);
		let _ = cli_json(&harness, &["editor", "mode", "enter-insert-after"]);
		let _ = cli_json(&harness, &["editor", "motion", "line-end"]);
		let _ = cli_json(&harness, &["editor", "edit", "insert", " world"]);
		let _ = cli_json(&harness, &["scene", "ensure"]);
		harness
	};
	let cli_snapshot: GlorpSnapshot =
		serde_json::from_value(cli_json(&cli_harness, &["get", "state"])).expect("CLI snapshot JSON should decode");

	assert_eq!(ipc.document_text, direct.document_text);
	assert_eq!(plugin_snapshot.document_text, direct.document_text);
	assert_eq!(cli_snapshot.document_text, direct.document_text);
	assert_eq!(ipc.config.editor.wrapping, direct.config.editor.wrapping);
	assert_eq!(plugin_snapshot.config.editor.wrapping, direct.config.editor.wrapping);
	assert_eq!(cli_snapshot.config.editor.wrapping, direct.config.editor.wrapping);
}

#[test]
fn cli_autodetects_gui_socket_e2e() {
	let mut harness = Harness::new();
	harness.start_server();
	let json = cli_json_repo_root(&harness, &["doc", "replace", "shared-socket"]);
	let outcome: GlorpOutcome = serde_json::from_value(json).expect("CLI outcome should decode");
	assert!(outcome.delta.text_changed);
	assert_eq!(document_text(&mut harness.ipc_client()), "shared-socket");
}

#[test]
fn persistence_smoke_test() {
	let harness = Harness::new();
	let paths = harness.paths.clone();
	let mut host = harness.runtime();
	host.execute(GlorpCommand::Config(ConfigCommand::Set {
		path: "editor.wrapping".to_owned(),
		value: GlorpValue::String("glyph".to_owned()),
	}))
	.expect("config set should succeed");
	host.execute(GlorpCommand::Config(ConfigCommand::Persist))
		.expect("persist should succeed");
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

	host.execute(GlorpCommand::Config(ConfigCommand::Set {
		path: "inspect.show_hitboxes".to_owned(),
		value: GlorpValue::Bool(true),
	}))
	.expect("config update should succeed");
	host.execute(GlorpCommand::Document(DocumentCommand::Replace {
		text: "event stream".to_owned(),
	}))
	.expect("document replace should succeed");
	host.execute(GlorpCommand::Scene(SceneCommand::Ensure))
		.expect("scene ensure should succeed");
	assert!(
		host.execute(GlorpCommand::Config(ConfigCommand::Set {
			path: "editor.wrapping".to_owned(),
			value: GlorpValue::String("invalid".to_owned()),
		}))
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
	assert!(first.revisions.config < second.revisions.editor || first.revisions.editor < second.revisions.editor);
	assert!(second.revisions.scene <= third.revisions.scene);
	assert_eq!(first.changed_config_paths, vec!["inspect.show_hitboxes".to_owned()]);
	assert!(first.delta.config_changed);
	assert!(second.delta.text_changed);
	assert!(third.delta.scene_changed);
}

#[test]
fn golden_transcript_smoke_test() {
	let mut harness = Harness::new();
	harness.start_server();

	let _ = cli_json(&harness, &["config", "set", "editor.wrapping", "word"]);
	let _ = cli_json(&harness, &["doc", "replace", "hello"]);
	let _ = cli_json(&harness, &["editor", "mode", "enter-insert-after"]);
	let _ = cli_json(&harness, &["editor", "motion", "line-end"]);
	let _ = cli_json(&harness, &["editor", "edit", "insert", " world"]);
	let _ = cli_json(&harness, &["scene", "ensure"]);
	let snapshot: GlorpSnapshot =
		serde_json::from_value(cli_json(&harness, &["get", "state"])).expect("snapshot JSON should decode");

	assert_eq!(snapshot.config.editor.wrapping, WrapChoice::Word);
	assert_eq!(snapshot.document_text.as_deref(), Some("hello world"));
	assert!(snapshot.revisions.editor > 0);
	assert!(snapshot.revisions.scene.is_some());
	assert!(snapshot.editor.undo_depth > 0);
	assert_eq!(snapshot.ui.active_tab, SidebarTab::Controls);
	assert_eq!(snapshot.ui.canvas_scroll_y, 0.0);
}

#[test]
fn selection_query_e2e() {
	let mut host = Harness::new().runtime();
	host.execute(GlorpCommand::Document(DocumentCommand::Replace {
		text: "alpha beta".to_owned(),
	}))
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
	host.execute(GlorpCommand::Document(DocumentCommand::Replace {
		text: "inspect me".to_owned(),
	}))
	.expect("replace should succeed");
	host.execute(GlorpCommand::Ui(UiCommand::SidebarSelect {
		tab: SidebarTab::Inspect,
	}))
	.expect("sidebar select should succeed");
	host.execute(GlorpCommand::Scene(SceneCommand::Ensure))
		.expect("scene ensure should succeed");
	host.execute(GlorpCommand::Ui(UiCommand::InspectTargetSelect {
		target: Some(CanvasTarget::Cluster(0)),
	}))
	.expect("inspect target select should succeed");

	let inspect = match host
		.query(GlorpQuery::InspectDetails { target: None })
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
	host.execute(GlorpCommand::Document(DocumentCommand::Replace {
		text: "one\ntwo\nthree".to_owned(),
	}))
	.expect("replace should succeed");

	let perf = match host
		.query(GlorpQuery::PerfDashboard)
		.expect("perf dashboard query should succeed")
	{
		GlorpQueryResult::PerfDashboard(perf) => perf,
		other => panic!("unexpected perf dashboard response: {other:?}"),
	};

	assert!(perf.overview.scene_ready);
	assert!(perf.overview.scene_revision.is_some());
	assert!(!perf.metrics.is_empty());
	assert_eq!(perf.metrics[0].label, "scene.build");
	assert!(perf.metrics[0].total_samples >= 1);
}

#[test]
fn cli_session_attach_and_event_polling_e2e() {
	let mut harness = Harness::new();
	harness.start_server();

	let session: GlorpSessionView = serde_json::from_value(cli_json_repo_root(&harness, &["session", "attach"]))
		.expect("session attach JSON should decode");
	assert_eq!(session.socket, harness.socket_path.display().to_string());
	assert!(session.capabilities.subscriptions);

	let stream: GlorpEventStreamView = serde_json::from_value(cli_json_repo_root(&harness, &["events", "subscribe"]))
		.expect("event stream JSON should decode");
	let _ = cli_json_repo_root(&harness, &["doc", "replace", "eventful"]);
	let event: Option<GlorpEvent> = serde_json::from_value(cli_json_repo_root(
		&harness,
		&["events", "next", &stream.token.to_string()],
	))
	.expect("event JSON should decode");

	match event {
		Some(GlorpEvent::Changed(outcome)) => assert!(outcome.delta.text_changed),
		other => panic!("unexpected event: {other:?}"),
	}

	let _ = cli_json_repo_root(&harness, &["events", "unsubscribe", &stream.token.to_string()]);
}

#[test]
fn nu_module_session_and_txn_e2e() {
	let mut harness = Harness::new();
	harness.start_server();
	let module_path = repo_root().join("nu/glorp.nu");

	let output = nu_command()
		.arg("-c")
		.arg(format!(
			r#"use "{}" *;
let session = (glorp session attach --socket "{}");
glorp txn [
  (glorp cmd doc replace "hello")
  (glorp cmd editor mode enter-insert-after)
  (glorp cmd editor motion line-end)
  (glorp cmd editor edit insert " world")
] --session $session | ignore;
glorp get document-text --session $session | to json -r"#,
			module_path.display(),
			harness.socket_path.display(),
		))
		.output()
		.expect("nu should run");

	assert!(
		output.status.success(),
		"nu stderr: {}",
		String::from_utf8_lossy(&output.stderr)
	);
	let document: String = serde_json::from_slice(&output.stdout).expect("nu output should decode");
	assert_eq!(document, "hello world");
}
