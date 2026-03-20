use {
	glorp_api::{EditorContextView, EditorMode, GlorpCallDescriptor, TextInput},
	glorp_gui::{GuiLaunchOptions, GuiRuntimeClient, GuiRuntimeSession},
	glorp_runtime::{GuiEditCommand, GuiEditRequest, GuiLayoutRequest},
	glorp_test_support::TestRepo,
	glorp_transport::default_socket_path,
	std::{
		fmt,
		time::{Duration, Instant},
	},
};

const DEFAULT_WARMUP: usize = 10;
const DEFAULT_SAMPLES: usize = 60;
const FRAME_BUDGET_MS: f64 = 16.7;
const WARNING_BUDGET_MS: f64 = 8.0;

#[test]
#[ignore = "manual perf harness: run with cargo test -p glorp_host --test gui_host_perf -- --ignored --nocapture"]
fn gui_host_perf_report() {
	let warmup = env_usize("GLORP_PERF_WARMUP", DEFAULT_WARMUP);
	let samples = env_usize("GLORP_PERF_SAMPLES", DEFAULT_SAMPLES);

	println!(
		"glorp gui<->host perf harness warmup={} samples={} frame_budget_ms={}",
		warmup, samples, FRAME_BUDGET_MS
	);

	for transport in [TransportKind::OwnedLocal, TransportKind::AttachedIpc] {
		println!("\n== {} ==", transport.label());
		let mut reports = vec![
			run_frame_poll(transport, warmup, samples),
			run_resize_reflow(transport, warmup, samples),
			run_edit_only(transport, warmup, samples),
			run_edit_with_frame_refresh(transport, warmup, samples),
			run_scene_fetch_cold(transport, warmup, samples),
			run_scene_fetch_not_modified(transport, warmup, samples),
		];
		if matches!(transport, TransportKind::AttachedIpc) {
			reports.push(run_push_propagation_large_invalidate(warmup, samples));
			reports.push(run_document_fetch_large(warmup, samples));
		}
		for report in reports {
			report.print();
		}
	}
}

#[derive(Clone, Copy)]
enum TransportKind {
	OwnedLocal,
	AttachedIpc,
}

impl TransportKind {
	const fn label(self) -> &'static str {
		match self {
			Self::OwnedLocal => "owned-local",
			Self::AttachedIpc => "attached-ipc",
		}
	}
}

struct PerfHarness {
	_repo: TestRepo,
	_owner: GuiRuntimeSession,
	_attached: Option<GuiRuntimeSession>,
	owner_client: Option<GuiRuntimeClient>,
	client: GuiRuntimeClient,
}

#[derive(Debug)]
struct ScenarioReport {
	name: &'static str,
	transport: TransportKind,
	samples: Vec<f64>,
}

impl ScenarioReport {
	fn print(&self) {
		let mut values = self.samples.clone();
		values.sort_by(f64::total_cmp);
		let mean = values.iter().sum::<f64>() / values.len() as f64;
		let p50 = percentile(&values, 0.50);
		let p95 = percentile(&values, 0.95);
		let p99 = percentile(&values, 0.99);
		let max = *values.last().unwrap_or(&0.0);
		let over_warning = values.iter().filter(|&&ms| ms >= WARNING_BUDGET_MS).count();
		let over_budget = values.iter().filter(|&&ms| ms >= FRAME_BUDGET_MS).count();

		println!(
			"{:<24} {:<13} mean={:>6.2}ms p50={:>6.2}ms p95={:>6.2}ms p99={:>6.2}ms max={:>6.2}ms warn={} budget={}",
			self.name,
			self.transport.label(),
			mean,
			p50,
			p95,
			p99,
			max,
			over_warning,
			over_budget,
		);
	}
}

impl PerfHarness {
	fn new(transport: TransportKind) -> Self {
		let harness = TestRepo::new("glorp-gui-perf");
		let options = GuiLaunchOptions {
			repo_root: harness.root.clone(),
			socket_path: default_socket_path(&harness.root),
		};

		match transport {
			TransportKind::OwnedLocal => {
				let (owner, client) =
					GuiRuntimeSession::connect_or_start(options).expect("owned GUI session should start");
				Self {
					_repo: harness,
					_owner: owner,
					_attached: None,
					owner_client: None,
					client,
				}
			}
			TransportKind::AttachedIpc => {
				let (owner, owner_client) =
					GuiRuntimeSession::connect_or_start(options.clone()).expect("owner GUI session should start");
				let (attached, client) =
					GuiRuntimeSession::connect_or_start(options).expect("attached GUI session should connect");
				Self {
					_repo: harness,
					_owner: owner,
					_attached: Some(attached),
					owner_client: Some(owner_client),
					client,
				}
			}
		}
	}

	fn seed_document(&mut self, text: &str) {
		let _ = glorp_api::calls::DocumentReplace::call(&mut self.client, TextInput { text: text.to_owned() })
			.expect("document replace should succeed");
		let _ = self.client.gui_frame().expect("gui frame should refresh");
	}
}

fn run_frame_poll(transport: TransportKind, warmup: usize, samples: usize) -> ScenarioReport {
	let mut harness = PerfHarness::new(transport);
	harness.seed_document(&large_document());

	for _ in 0..warmup {
		let _ = harness.client.gui_frame().expect("warmup frame should load");
	}

	let mut results = Vec::with_capacity(samples);
	for _ in 0..samples {
		results.push(measure(|| {
			let _ = harness.client.gui_frame().expect("sample frame should load");
		}));
	}

	ScenarioReport {
		name: "frame-poll",
		transport,
		samples: results,
	}
}

fn run_resize_reflow(transport: TransportKind, warmup: usize, samples: usize) -> ScenarioReport {
	let mut harness = PerfHarness::new(transport);
	harness.seed_document(&large_document());
	let widths = [320.0_f32, 420.0, 540.0, 680.0, 460.0];

	for step in 0..warmup {
		harness.client.set_layout_width(widths[step % widths.len()]);
		let _ = harness.client.gui_frame().expect("warmup resize frame should load");
	}

	let mut results = Vec::with_capacity(samples);
	for step in 0..samples {
		results.push(measure(|| {
			harness.client.set_layout_width(widths[step % widths.len()]);
			let _ = harness.client.gui_frame().expect("resize frame should load");
		}));
	}

	ScenarioReport {
		name: "resize-reflow",
		transport,
		samples: results,
	}
}

fn run_edit_only(transport: TransportKind, warmup: usize, samples: usize) -> ScenarioReport {
	let mut harness = PerfHarness::new(transport);
	let text = large_document();
	let base_len = text.len() as u64;
	harness.seed_document(&text);
	let mut context = insert_context(base_len);

	for step in 0..warmup {
		context = run_edit_step(&mut harness.client, context, step);
	}

	let mut results = Vec::with_capacity(samples);
	for step in 0..samples {
		let started = Instant::now();
		context = run_edit_step(&mut harness.client, context, step);
		results.push(elapsed_ms(started.elapsed()));
	}

	ScenarioReport {
		name: "edit-only",
		transport,
		samples: results,
	}
}

fn run_edit_with_frame_refresh(transport: TransportKind, warmup: usize, samples: usize) -> ScenarioReport {
	let mut harness = PerfHarness::new(transport);
	let text = large_document();
	let base_len = text.len() as u64;
	harness.seed_document(&text);
	let mut context = insert_context(base_len);

	for step in 0..warmup {
		context = run_edit_step(&mut harness.client, context, step);
		let _ = harness.client.gui_frame().expect("warmup post-edit frame should load");
	}

	let mut results = Vec::with_capacity(samples);
	for step in 0..samples {
		let started = Instant::now();
		context = run_edit_step(&mut harness.client, context, step);
		let _ = harness.client.gui_frame().expect("post-edit frame should load");
		results.push(elapsed_ms(started.elapsed()));
	}

	ScenarioReport {
		name: "edit+frame",
		transport,
		samples: results,
	}
}

fn run_scene_fetch_cold(transport: TransportKind, warmup: usize, samples: usize) -> ScenarioReport {
	let mut harness = PerfHarness::new(transport);
	let doc_a = large_document();
	let doc_b = alternate_document();

	for step in 0..warmup {
		invalidate_scene(&mut harness.client, if step % 2 == 0 { &doc_a } else { &doc_b });
		let _ = harness
			.client
			.scene_fetch(0)
			.expect("warmup scene fetch should succeed");
	}

	let mut results = Vec::with_capacity(samples);
	for step in 0..samples {
		invalidate_scene(&mut harness.client, if step % 2 == 0 { &doc_a } else { &doc_b });
		results.push(measure(|| {
			let _ = harness.client.scene_fetch(0).expect("scene fetch should succeed");
		}));
	}

	ScenarioReport {
		name: "scene-fetch-cold-binary",
		transport,
		samples: results,
	}
}

fn run_scene_fetch_not_modified(transport: TransportKind, warmup: usize, samples: usize) -> ScenarioReport {
	let mut harness = PerfHarness::new(transport);
	harness.seed_document(&large_document());
	let _ = harness
		.client
		.scene_fetch(0)
		.expect("initial scene fetch should succeed");
	let revision = harness
		.client
		.gui_frame()
		.expect("scene frame should load")
		.scene_summary
		.revision;

	for _ in 0..warmup {
		let _ = harness
			.client
			.scene_fetch(revision)
			.expect("warmup scene fetch should succeed");
	}

	let mut results = Vec::with_capacity(samples);
	for _ in 0..samples {
		results.push(measure(|| {
			let _ = harness
				.client
				.scene_fetch(revision)
				.expect("scene fetch should succeed");
		}));
	}

	ScenarioReport {
		name: "scene-fetch-not-modified",
		transport,
		samples: results,
	}
}

fn run_push_propagation_large_invalidate(warmup: usize, samples: usize) -> ScenarioReport {
	let mut harness = PerfHarness::new(TransportKind::AttachedIpc);
	let doc_a = large_document();
	let doc_b = alternate_document();
	let (owner, client) = {
		let PerfHarness {
			owner_client, client, ..
		} = &mut harness;
		(
			owner_client
				.as_mut()
				.expect("attached transport should keep an owner client"),
			client,
		)
	};
	let _ = client.gui_frame().expect("attached frame should load");

	for step in 0..warmup {
		let _ = glorp_api::calls::DocumentReplace::call(
			owner,
			TextInput {
				text: if step % 2 == 0 { doc_a.clone() } else { doc_b.clone() },
			},
		)
		.expect("warmup document replace should succeed");
		let _ = wait_for_pushed_delta(client).expect("warmup pushed delta should arrive");
	}

	let mut results = Vec::with_capacity(samples);
	for step in 0..samples {
		let started = Instant::now();
		let _ = glorp_api::calls::DocumentReplace::call(
			owner,
			TextInput {
				text: if step % 2 == 0 { doc_a.clone() } else { doc_b.clone() },
			},
		)
		.expect("document replace should succeed");
		let _ = wait_for_pushed_delta(client).expect("pushed delta should arrive");
		results.push(elapsed_ms(started.elapsed()));
	}

	ScenarioReport {
		name: "push-propagation-large-invalidate",
		transport: TransportKind::AttachedIpc,
		samples: results,
	}
}

fn run_document_fetch_large(warmup: usize, samples: usize) -> ScenarioReport {
	let mut harness = PerfHarness::new(TransportKind::AttachedIpc);
	let doc_a = large_document();
	let doc_b = alternate_document();
	let (owner, client) = {
		let PerfHarness {
			owner_client, client, ..
		} = &mut harness;
		(
			owner_client
				.as_mut()
				.expect("attached transport should keep an owner client"),
			client,
		)
	};
	let _ = client.gui_frame().expect("attached frame should load");

	for step in 0..warmup {
		let text = if step % 2 == 0 { doc_a.clone() } else { doc_b.clone() };
		let revision = replace_and_fetch_large(owner, client, text);
		let _ = client
			.document_fetch(revision)
			.expect("warmup document fetch should succeed");
	}

	let mut results = Vec::with_capacity(samples);
	for step in 0..samples {
		let text = if step % 2 == 0 { doc_a.clone() } else { doc_b.clone() };
		let revision = replace_and_fetch_large(owner, client, text);
		results.push(measure(|| {
			let _ = client.document_fetch(revision).expect("document fetch should succeed");
		}));
	}

	ScenarioReport {
		name: "document-fetch-large",
		transport: TransportKind::AttachedIpc,
		samples: results,
	}
}

fn invalidate_scene(client: &mut GuiRuntimeClient, text: &str) {
	let _ = glorp_api::calls::DocumentReplace::call(client, TextInput { text: text.to_owned() })
		.expect("document replace should succeed");
}

fn run_edit_step(client: &mut GuiRuntimeClient, context: EditorContextView, step: usize) -> EditorContextView {
	let command = if step % 2 == 0 {
		GuiEditCommand::InsertText("x".to_owned())
	} else {
		GuiEditCommand::Backspace
	};
	client
		.gui_edit(GuiEditRequest {
			layout: GuiLayoutRequest { layout_width: 540.0 },
			context,
			command,
		})
		.expect("gui edit should succeed")
		.next_context
}

fn wait_for_pushed_delta(client: &mut GuiRuntimeClient) -> Option<glorp_runtime::GuiSharedDelta> {
	for _ in 0..200 {
		if let Some(delta) = client.drain_events().into_iter().find_map(|message| match message {
			glorp_runtime::GuiSessionHostMessage::Changed(delta) => Some(delta),
			_ => None,
		}) {
			return Some(delta);
		}
		std::thread::sleep(Duration::from_millis(1));
	}
	None
}

fn replace_and_fetch_large(owner: &mut GuiRuntimeClient, client: &mut GuiRuntimeClient, text: String) -> u64 {
	let _ =
		glorp_api::calls::DocumentReplace::call(owner, TextInput { text }).expect("document replace should succeed");
	wait_for_pushed_delta(client)
		.expect("pushed delta should arrive")
		.document_sync
		.expect("large document replace should produce a document sync")
		.revision
}

fn insert_context(selection_head: u64) -> EditorContextView {
	EditorContextView {
		mode: EditorMode::Insert,
		selection: None,
		selection_head: Some(selection_head),
	}
}

fn large_document() -> String {
	repeated_block(
		"alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron pi rho sigma tau\n",
		700,
	)
}

fn alternate_document() -> String {
	repeated_block(
		"alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron pi rho sigma alt\n",
		700,
	)
}

fn repeated_block(line: &str, lines: usize) -> String {
	let mut out = String::with_capacity(line.len() * lines);
	for _ in 0..lines {
		out.push_str(line);
	}
	out
}

fn env_usize(key: &str, default: usize) -> usize {
	std::env::var(key)
		.ok()
		.and_then(|value| value.parse::<usize>().ok())
		.filter(|&value| value > 0)
		.unwrap_or(default)
}

fn measure(run: impl FnOnce()) -> f64 {
	let started = Instant::now();
	run();
	elapsed_ms(started.elapsed())
}

fn elapsed_ms(duration: Duration) -> f64 {
	duration.as_secs_f64() * 1000.0
}

fn percentile(sorted: &[f64], quantile: f64) -> f64 {
	if sorted.is_empty() {
		return 0.0;
	}

	let index = ((sorted.len() - 1) as f64 * quantile).round() as usize;
	sorted[index.min(sorted.len() - 1)]
}

impl fmt::Debug for TransportKind {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(self.label())
	}
}
