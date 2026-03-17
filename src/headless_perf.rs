use {
	crate::{EditorApp, PerfScenario, perf::PerfDashboard},
	iced::{
		Color, Font, Pixels, Size, Theme,
		advanced::{
			graphics::Viewport,
			renderer::{Headless, Style},
		},
		mouse,
	},
	iced_runtime::{UserInterface, user_interface},
	pollster::block_on,
	std::{env, fmt::Write as _, process::ExitCode},
};

const DEFAULT_WARMUP_FRAMES: usize = 30;
const DEFAULT_SAMPLE_FRAMES: usize = 180;
const VIEWPORT_SCALE_FACTOR: f32 = 1.0;
const VIEWPORT_PHYSICAL_WIDTH: u32 = 1600;
const VIEWPORT_PHYSICAL_HEIGHT: u32 = 1000;
const VIEWPORT_LOGICAL_SIZE: Size = Size::new(1600.0, 1000.0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PerfCliConfig {
	pub(crate) scenario: PerfScenario,
	pub(crate) warmup_frames: usize,
	pub(crate) sample_frames: usize,
}

pub(crate) fn run_from_env() -> Option<ExitCode> {
	let args = env::args().skip(1).collect::<Vec<_>>();
	if args.iter().any(|arg| arg == "--help" || arg == "-h") {
		println!("{}", usage());
		return Some(ExitCode::SUCCESS);
	}

	let config = match parse_args(&args) {
		Ok(Some(config)) => config,
		Ok(None) => return None,
		Err(message) => return Some(report_error(&message)),
	};

	crate::init_tracing();

	run(config).map_or_else(
		|message| Some(report_error(&message)),
		|report| {
			println!("{report}");
			Some(ExitCode::SUCCESS)
		},
	)
}

fn parse_args(args: &[String]) -> Result<Option<PerfCliConfig>, String> {
	if args.is_empty() {
		return Ok(None);
	}

	let mut scenario = None;
	let mut warmup_frames = DEFAULT_WARMUP_FRAMES;
	let mut sample_frames = DEFAULT_SAMPLE_FRAMES;
	let mut saw_perf_flag = false;
	let mut index = 0;

	while index < args.len() {
		match args[index].as_str() {
			"--perf-scenario" => {
				saw_perf_flag = true;
				let value = required_arg(args, index, "--perf-scenario")?;
				scenario = Some(
					PerfScenario::parse_label(value)
						.ok_or_else(|| format!("unknown perf scenario `{value}`\n\n{}", usage()))?,
				);
				index += 2;
			}
			"--warmup" => {
				saw_perf_flag = true;
				let value = required_arg(args, index, "--warmup")?;
				warmup_frames = parse_count("--warmup", value)?;
				index += 2;
			}
			"--samples" => {
				saw_perf_flag = true;
				let value = required_arg(args, index, "--samples")?;
				sample_frames = parse_count("--samples", value)?;
				index += 2;
			}
			flag => return Err(format!("unknown argument `{flag}`\n\n{}", usage())),
		}
	}

	match scenario {
		Some(scenario) => Ok(Some(PerfCliConfig {
			scenario,
			warmup_frames,
			sample_frames,
		})),
		None if saw_perf_flag => Err(format!(
			"--perf-scenario is required when using perf flags\n\n{}",
			usage()
		)),
		None => Ok(None),
	}
}

fn required_arg<'a>(args: &'a [String], index: usize, flag: &str) -> Result<&'a str, String> {
	args.get(index + 1)
		.map(String::as_str)
		.ok_or_else(|| format!("{flag} requires a value"))
}

fn parse_count(flag: &str, value: &str) -> Result<usize, String> {
	match value.parse::<usize>() {
		Ok(count @ 1..) => Ok(count),
		_ => Err(format!("{flag} expects a positive integer, got `{value}`")),
	}
}

fn usage() -> String {
	let scenarios = PerfScenario::ALL
		.into_iter()
		.map(PerfScenario::label)
		.collect::<Vec<_>>()
		.join("|");

	format!("Usage: glorp [--perf-scenario <{scenarios}>] [--warmup <frames>] [--samples <frames>]")
}

fn report_error(message: &str) -> ExitCode {
	eprintln!("{message}");
	ExitCode::FAILURE
}

fn run(config: PerfCliConfig) -> Result<String, String> {
	let mut harness = Harness::new(config.scenario)?;

	for _ in 0..config.warmup_frames {
		harness.render_frame();
	}

	harness.reset_perf_monitor();

	for step in 0..config.sample_frames {
		harness.step(step);
		harness.render_frame();
	}

	let capture = harness.capture_screenshot();
	let dashboard = harness.dashboard();

	Ok(build_report_json(config, &harness.renderer_name, capture, &dashboard))
}

struct Harness {
	app: EditorApp,
	renderer: iced::Renderer,
	renderer_name: String,
	cache: user_interface::Cache,
	viewport_physical: Size<u32>,
	viewport_logical: Size,
	theme: Theme,
	scenario: PerfScenario,
}

impl Harness {
	fn new(scenario: PerfScenario) -> Result<Self, String> {
		let backend = env::var("GLORP_HEADLESS_BACKEND").ok();
		let renderer = block_on(<iced::Renderer as Headless>::new(
			iced::advanced::renderer::Settings {
				default_font: Font::DEFAULT,
				default_text_size: Pixels::from(16),
			},
			backend.as_deref(),
		))
		.ok_or_else(|| "failed to create headless renderer".to_string())?;
		let renderer_name = renderer.name();
		let viewport_physical = Size::new(VIEWPORT_PHYSICAL_WIDTH, VIEWPORT_PHYSICAL_HEIGHT);
		let app = EditorApp::headless();

		let mut harness = Self {
			app,
			renderer,
			renderer_name,
			cache: user_interface::Cache::default(),
			viewport_physical,
			viewport_logical: VIEWPORT_LOGICAL_SIZE,
			theme: Theme::TokyoNightStorm,
			scenario,
		};
		harness.app.headless_driver().configure_perf_scenario(scenario);
		Ok(harness)
	}

	fn step(&mut self, step: usize) {
		self.app.headless_driver().run_perf_step(self.scenario, step);
	}

	fn render_frame(&mut self) {
		self.draw_frame();
		self.app.flush_perf_metrics();
	}

	fn draw_frame(&mut self) {
		let (cache, build_duration, draw_duration) = {
			let build_started = std::time::Instant::now();
			let mut user_interface = UserInterface::build(
				self.app.headless_view(),
				self.viewport_logical,
				std::mem::take(&mut self.cache),
				&mut self.renderer,
			);
			let build_duration = build_started.elapsed();

			let draw_started = std::time::Instant::now();
			user_interface.draw(
				&mut self.renderer,
				&self.theme,
				&Style {
					text_color: Color::WHITE,
				},
				mouse::Cursor::Unavailable,
			);
			let draw_duration = draw_started.elapsed();

			(user_interface.into_cache(), build_duration, draw_duration)
		};

		self.cache = cache;
		self.app.record_headless_ui_build(build_duration);
		self.app.record_headless_ui_draw(draw_duration);
	}

	fn capture_screenshot(&mut self) -> CaptureSummary {
		let started = std::time::Instant::now();
		let viewport = Viewport::with_physical_size(self.viewport_physical, VIEWPORT_SCALE_FACTOR);
		let bytes = self.renderer.screenshot(&viewport, Color::BLACK).len();
		CaptureSummary {
			mode: "final-only",
			bytes,
			capture_ms: started.elapsed().as_secs_f64() * 1000.0,
		}
	}

	fn reset_perf_monitor(&mut self) {
		self.app.reset_perf_monitor();
	}

	fn dashboard(&mut self) -> PerfDashboard {
		self.app.perf_dashboard()
	}
}

#[derive(Debug, Clone, Copy)]
struct CaptureSummary {
	mode: &'static str,
	bytes: usize,
	capture_ms: f64,
}

fn build_report_json(
	config: PerfCliConfig, renderer_name: &str, capture: CaptureSummary, dashboard: &PerfDashboard,
) -> String {
	let mut json = String::new();
	json.push_str("{\n");
	append_report_header(&mut json, config, renderer_name);
	append_capture(&mut json, capture);
	append_overview(&mut json, dashboard);
	append_frame_pacing(&mut json, dashboard);
	append_hot_paths(&mut json, dashboard);
	json.push_str("}\n");
	json
}

fn append_report_header(json: &mut String, config: PerfCliConfig, renderer_name: &str) {
	let _ = writeln!(json, "  \"scenario\": {},", json_string(config.scenario.label()));
	let _ = writeln!(json, "  \"driver\": {},", json_string(config.scenario.driver()));
	let _ = writeln!(json, "  \"renderer\": {},", json_string(renderer_name));
	let _ = writeln!(
		json,
		"  \"build_profile\": {},",
		json_string(if cfg!(debug_assertions) { "debug" } else { "release" })
	);
	json.push_str("  \"viewport\": {\n");
	let _ = writeln!(json, "    \"width\": {VIEWPORT_PHYSICAL_WIDTH},");
	let _ = writeln!(json, "    \"height\": {VIEWPORT_PHYSICAL_HEIGHT}");
	json.push_str("  },\n");
	json.push_str("  \"sampling\": {\n");
	let _ = writeln!(json, "    \"warmup_frames\": {},", config.warmup_frames);
	let _ = writeln!(json, "    \"sample_frames\": {}", config.sample_frames);
	json.push_str("  },\n");
}

fn append_capture(json: &mut String, capture: CaptureSummary) {
	json.push_str("  \"capture\": {\n");
	let _ = writeln!(json, "    \"mode\": {},", json_string(capture.mode));
	let _ = writeln!(json, "    \"bytes\": {},", capture.bytes);
	let _ = writeln!(json, "    \"capture_ms\": {}", json_float(capture.capture_ms));
	json.push_str("  },\n");
}

fn append_overview(json: &mut String, dashboard: &PerfDashboard) {
	json.push_str("  \"overview\": {\n");
	let _ = writeln!(
		json,
		"    \"editor_mode\": {},",
		json_string(&dashboard.overview.editor_mode.to_string())
	);
	let _ = writeln!(json, "    \"editor_bytes\": {},", dashboard.overview.editor_bytes);
	let _ = writeln!(json, "    \"editor_chars\": {},", dashboard.overview.editor_chars);
	let _ = writeln!(json, "    \"line_count\": {},", dashboard.overview.line_count);
	let _ = writeln!(json, "    \"run_count\": {},", dashboard.overview.run_count);
	let _ = writeln!(json, "    \"glyph_count\": {},", dashboard.overview.glyph_count);
	let _ = writeln!(json, "    \"cluster_count\": {},", dashboard.overview.cluster_count);
	let _ = writeln!(json, "    \"font_count\": {},", dashboard.overview.font_count);
	let _ = writeln!(json, "    \"warning_count\": {},", dashboard.overview.warning_count);
	let _ = writeln!(
		json,
		"    \"scene_width\": {},",
		json_float(f64::from(dashboard.overview.scene_width))
	);
	let _ = writeln!(
		json,
		"    \"scene_height\": {},",
		json_float(f64::from(dashboard.overview.scene_height))
	);
	let _ = writeln!(
		json,
		"    \"layout_width\": {}",
		json_float(f64::from(dashboard.overview.layout_width))
	);
	json.push_str("  },\n");
}

fn append_frame_pacing(json: &mut String, dashboard: &PerfDashboard) {
	json.push_str("  \"frame_pacing\": {\n");
	let _ = writeln!(
		json,
		"    \"fps\": {},",
		json_float(f64::from(dashboard.frame_pacing.fps))
	);
	let _ = writeln!(
		json,
		"    \"last_ms\": {},",
		json_float(f64::from(dashboard.frame_pacing.last_ms))
	);
	let _ = writeln!(
		json,
		"    \"avg_ms\": {},",
		json_float(f64::from(dashboard.frame_pacing.avg_ms))
	);
	let _ = writeln!(
		json,
		"    \"max_ms\": {},",
		json_float(f64::from(dashboard.frame_pacing.max_ms))
	);
	let _ = writeln!(json, "    \"total_draws\": {},", dashboard.frame_pacing.total_draws);
	let _ = writeln!(json, "    \"over_budget\": {},", dashboard.frame_pacing.over_budget);
	let _ = writeln!(json, "    \"severe_jank\": {},", dashboard.frame_pacing.severe_jank);
	let _ = writeln!(json, "    \"cache_hits\": {},", dashboard.frame_pacing.cache_hits);
	let _ = writeln!(json, "    \"cache_misses\": {},", dashboard.frame_pacing.cache_misses);
	let _ = writeln!(
		json,
		"    \"recent_ms\": {}",
		json_numbers(&dashboard.frame_pacing.recent_ms)
	);
	json.push_str("  },\n");
}

fn append_hot_paths(json: &mut String, dashboard: &PerfDashboard) {
	json.push_str("  \"hot_paths\": [\n");
	for (index, summary) in dashboard.hot_paths.iter().enumerate() {
		let prefix = if index == 0 { "" } else { ",\n" };
		let _ = write!(
			json,
			"{prefix}    {{\"label\": {}, \"last_ms\": {}, \"avg_ms\": {}, \"p95_ms\": {}, \"max_ms\": {}, \"total_samples\": {}, \"over_warning\": {}, \"over_budget\": {}}}",
			json_string(summary.label),
			json_float(f64::from(summary.last_ms)),
			json_float(f64::from(summary.avg_ms)),
			json_float(f64::from(summary.p95_ms)),
			json_float(f64::from(summary.max_ms)),
			summary.total_samples,
			summary.over_warning,
			summary.over_budget,
		);
	}
	json.push_str("\n  ]\n");
}

fn json_numbers(values: &[f32]) -> String {
	let mut json = String::with_capacity((values.len() * 8) + 2);
	json.push('[');
	for (index, value) in values.iter().enumerate() {
		if index > 0 {
			json.push_str(", ");
		}
		json.push_str(&json_float(f64::from(*value)));
	}
	json.push(']');
	json
}

fn json_float(value: f64) -> String {
	format!("{value:.3}")
}

fn json_string(value: &str) -> String {
	let mut json = String::with_capacity(value.len() + 2);
	json.push('"');
	for ch in value.chars() {
		match ch {
			'\\' => json.push_str("\\\\"),
			'"' => json.push_str("\\\""),
			'\n' => json.push_str("\\n"),
			'\r' => json.push_str("\\r"),
			'\t' => json.push_str("\\t"),
			_ => json.extend(ch.escape_default()),
		}
	}
	json.push('"');
	json
}

#[cfg(test)]
mod tests {
	use {
		super::{DEFAULT_SAMPLE_FRAMES, DEFAULT_WARMUP_FRAMES, parse_args},
		crate::PerfScenario,
	};

	fn strings(values: &[&str]) -> Vec<String> {
		values.iter().map(ToString::to_string).collect()
	}

	#[test]
	fn parse_args_reads_perf_flags() {
		let config = parse_args(&strings(&[
			"--perf-scenario",
			"tall-inspect",
			"--warmup",
			"12",
			"--samples",
			"48",
		]))
		.expect("args should parse")
		.expect("perf mode should be selected");

		assert_eq!(config.scenario, PerfScenario::TallInspect);
		assert_eq!(config.warmup_frames, 12);
		assert_eq!(config.sample_frames, 48);
	}

	#[test]
	fn parse_args_defaults_frame_counts() {
		let config = parse_args(&strings(&["--perf-scenario", "tall"]))
			.expect("args should parse")
			.expect("perf mode should be selected");

		assert_eq!(config.scenario, PerfScenario::Tall);
		assert_eq!(config.warmup_frames, DEFAULT_WARMUP_FRAMES);
		assert_eq!(config.sample_frames, DEFAULT_SAMPLE_FRAMES);
	}

	#[test]
	fn parse_args_accepts_scripted_perf_scenarios() {
		let config = parse_args(&strings(&["--perf-scenario", "resize-reflow", "--samples", "24"]))
			.expect("args should parse")
			.expect("perf mode should be selected");

		assert_eq!(config.scenario, PerfScenario::ResizeReflow);
		assert_eq!(config.sample_frames, 24);
	}

	#[test]
	fn parse_args_rejects_zero_and_unknown_scenarios() {
		let zero = parse_args(&strings(&["--perf-scenario", "default", "--samples", "0"]))
			.expect_err("zero sample count should fail");
		assert!(zero.contains("--samples expects a positive integer"));

		let unknown = parse_args(&strings(&["--perf-scenario", "nope"])).expect_err("unknown scenario should fail");
		assert!(unknown.contains("unknown perf scenario `nope`"));
	}

	#[test]
	fn parse_args_requires_scenario_when_perf_flags_are_present() {
		let error = parse_args(&strings(&["--samples", "12"])).expect_err("perf flags without a scenario should fail");

		assert!(error.contains("--perf-scenario is required"));
	}
}
