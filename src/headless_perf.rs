use {
	crate::{HeadlessScenario, Playground, perf::PerfDashboard},
	iced::{
		Color, Font, Pixels, Size, Theme,
		advanced::renderer::{Headless, Style},
		mouse,
	},
	iced_runtime::{UserInterface, user_interface},
	pollster::block_on,
	std::{env, process::ExitCode},
};

const DEFAULT_WARMUP_FRAMES: usize = 30;
const DEFAULT_SAMPLE_FRAMES: usize = 180;
const VIEWPORT_SCALE_FACTOR: f32 = 1.0;
const VIEWPORT_PHYSICAL_WIDTH: u32 = 1600;
const VIEWPORT_PHYSICAL_HEIGHT: u32 = 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PerfCliConfig {
	pub(crate) scenario: HeadlessScenario,
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
		Err(message) => {
			eprintln!("{message}");
			return Some(ExitCode::FAILURE);
		}
	};

	crate::init_tracing();

	match run(config) {
		Ok(report) => {
			println!("{report}");
			Some(ExitCode::SUCCESS)
		}
		Err(message) => {
			eprintln!("{message}");
			Some(ExitCode::FAILURE)
		}
	}
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
				let value = args
					.get(index + 1)
					.ok_or_else(|| "--perf-scenario requires a value".to_string())?;
				scenario = Some(
					HeadlessScenario::parse_label(value)
						.ok_or_else(|| format!("unknown perf scenario `{value}`\n\n{}", usage()))?,
				);
				index += 2;
			}
			"--warmup" => {
				saw_perf_flag = true;
				let value = args
					.get(index + 1)
					.ok_or_else(|| "--warmup requires a value".to_string())?;
				warmup_frames = parse_count("--warmup", value)?;
				index += 2;
			}
			"--samples" => {
				saw_perf_flag = true;
				let value = args
					.get(index + 1)
					.ok_or_else(|| "--samples requires a value".to_string())?;
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

fn parse_count(flag: &str, value: &str) -> Result<usize, String> {
	let count = value
		.parse::<usize>()
		.map_err(|_| format!("{flag} expects a positive integer, got `{value}`"))?;

	if count == 0 {
		return Err(format!("{flag} expects a positive integer, got `0`"));
	}

	Ok(count)
}

fn usage() -> String {
	format!(
		"Usage: glorp [--perf-scenario <{}>] [--warmup <frames>] [--samples <frames>]",
		HeadlessScenario::ALL
			.into_iter()
			.map(HeadlessScenario::label)
			.collect::<Vec<_>>()
			.join("|")
	)
}

fn run(config: PerfCliConfig) -> Result<String, String> {
	let mut harness = Harness::new(config.scenario)?;

	for _ in 0..config.warmup_frames {
		let _ = harness.render_frame();
	}

	harness.reset_perf_monitor();

	let screenshot_bytes = (0..config.sample_frames)
		.map(|_| harness.render_frame())
		.collect::<Vec<_>>();
	let dashboard = harness.dashboard();

	Ok(build_report_json(
		config,
		&harness.renderer_name,
		screenshot_summary(&screenshot_bytes),
		&dashboard,
	))
}

struct Harness {
	playground: Playground,
	renderer: iced::Renderer,
	renderer_name: String,
	cache: user_interface::Cache,
	viewport_physical: Size<u32>,
	viewport_logical: Size,
	theme: Theme,
}

impl Harness {
	fn new(scenario: HeadlessScenario) -> Result<Self, String> {
		let backend = env::var("GLORP_HEADLESS_BACKEND").ok();
		let renderer = block_on(<iced::Renderer as Headless>::new(
			Font::DEFAULT,
			Pixels::from(16),
			backend.as_deref(),
		))
		.ok_or_else(|| "failed to create headless renderer".to_string())?;
		let renderer_name = renderer.name();
		let viewport_physical = Size::new(VIEWPORT_PHYSICAL_WIDTH, VIEWPORT_PHYSICAL_HEIGHT);
		let viewport_logical = Size::new(
			VIEWPORT_PHYSICAL_WIDTH as f32 / VIEWPORT_SCALE_FACTOR,
			VIEWPORT_PHYSICAL_HEIGHT as f32 / VIEWPORT_SCALE_FACTOR,
		);
		let playground = Playground::headless();

		let mut harness = Self {
			playground,
			renderer,
			renderer_name,
			cache: user_interface::Cache::default(),
			viewport_physical,
			viewport_logical,
			theme: Theme::TokyoNightStorm,
		};
		harness.playground.configure_headless_scenario(scenario);
		Ok(harness)
	}

	fn render_frame(&mut self) -> usize {
		let mut user_interface = UserInterface::build(
			self.playground.headless_view(),
			self.viewport_logical,
			std::mem::take(&mut self.cache),
			&mut self.renderer,
		);

		user_interface.draw(
			&mut self.renderer,
			&self.theme,
			&Style {
				text_color: Color::WHITE,
			},
			mouse::Cursor::Unavailable,
		);

		self.cache = user_interface.into_cache();

		let bytes = self
			.renderer
			.screenshot(self.viewport_physical, VIEWPORT_SCALE_FACTOR, Color::BLACK)
			.len();
		self.playground.flush_perf_metrics();
		bytes
	}

	fn reset_perf_monitor(&mut self) {
		self.playground.reset_perf_monitor();
	}

	fn dashboard(&self) -> PerfDashboard {
		self.playground.perf_dashboard()
	}
}

#[derive(Debug, Clone, Copy)]
struct ScreenshotSummary {
	last: usize,
	avg: f32,
	p95: usize,
	max: usize,
}

fn screenshot_summary(samples: &[usize]) -> ScreenshotSummary {
	let mut sorted = samples.to_vec();
	sorted.sort_unstable();

	let last = samples.last().copied().unwrap_or_default();
	let avg = if samples.is_empty() {
		0.0
	} else {
		samples.iter().copied().sum::<usize>() as f32 / samples.len() as f32
	};
	let p95_index = sorted
		.len()
		.checked_sub(1)
		.map(|last_index| ((last_index as f32) * 0.95).round() as usize)
		.unwrap_or_default();
	let p95 = sorted.get(p95_index).copied().unwrap_or_default();
	let max = sorted.last().copied().unwrap_or_default();

	ScreenshotSummary { last, avg, p95, max }
}

fn build_report_json(
	config: PerfCliConfig, renderer_name: &str, screenshots: ScreenshotSummary, dashboard: &PerfDashboard,
) -> String {
	let mut json = String::new();
	json.push_str("{\n");
	json.push_str(&format!("  \"scenario\": {},\n", json_string(config.scenario.label())));
	json.push_str(&format!("  \"renderer\": {},\n", json_string(renderer_name)));
	json.push_str(&format!(
		"  \"build_profile\": {},\n",
		json_string(if cfg!(debug_assertions) { "debug" } else { "release" })
	));
	json.push_str("  \"viewport\": {\n");
	json.push_str(&format!("    \"width\": {VIEWPORT_PHYSICAL_WIDTH},\n"));
	json.push_str(&format!("    \"height\": {VIEWPORT_PHYSICAL_HEIGHT}\n"));
	json.push_str("  },\n");
	json.push_str("  \"sampling\": {\n");
	json.push_str(&format!("    \"warmup_frames\": {},\n", config.warmup_frames));
	json.push_str(&format!("    \"sample_frames\": {}\n", config.sample_frames));
	json.push_str("  },\n");
	json.push_str("  \"screenshots\": {\n");
	json.push_str(&format!("    \"last_bytes\": {},\n", screenshots.last));
	json.push_str(&format!("    \"avg_bytes\": {},\n", json_f32(screenshots.avg)));
	json.push_str(&format!("    \"p95_bytes\": {},\n", screenshots.p95));
	json.push_str(&format!("    \"max_bytes\": {}\n", screenshots.max));
	json.push_str("  },\n");
	json.push_str("  \"overview\": {\n");
	json.push_str(&format!(
		"    \"editor_mode\": {},\n",
		json_string(&dashboard.overview.editor_mode.to_string())
	));
	json.push_str(&format!("    \"editor_bytes\": {},\n", dashboard.overview.editor_bytes));
	json.push_str(&format!("    \"editor_chars\": {},\n", dashboard.overview.editor_chars));
	json.push_str(&format!("    \"line_count\": {},\n", dashboard.overview.line_count));
	json.push_str(&format!("    \"run_count\": {},\n", dashboard.overview.run_count));
	json.push_str(&format!("    \"glyph_count\": {},\n", dashboard.overview.glyph_count));
	json.push_str(&format!(
		"    \"cluster_count\": {},\n",
		dashboard.overview.cluster_count
	));
	json.push_str(&format!("    \"font_count\": {},\n", dashboard.overview.font_count));
	json.push_str(&format!(
		"    \"warning_count\": {},\n",
		dashboard.overview.warning_count
	));
	json.push_str(&format!(
		"    \"scene_width\": {},\n",
		json_f32(dashboard.overview.scene_width)
	));
	json.push_str(&format!(
		"    \"scene_height\": {},\n",
		json_f32(dashboard.overview.scene_height)
	));
	json.push_str(&format!(
		"    \"layout_width\": {}\n",
		json_f32(dashboard.overview.layout_width)
	));
	json.push_str("  },\n");
	json.push_str("  \"frame_pacing\": {\n");
	json.push_str(&format!("    \"fps\": {},\n", json_f32(dashboard.frame_pacing.fps)));
	json.push_str(&format!(
		"    \"last_ms\": {},\n",
		json_f32(dashboard.frame_pacing.last_ms)
	));
	json.push_str(&format!(
		"    \"avg_ms\": {},\n",
		json_f32(dashboard.frame_pacing.avg_ms)
	));
	json.push_str(&format!(
		"    \"max_ms\": {},\n",
		json_f32(dashboard.frame_pacing.max_ms)
	));
	json.push_str(&format!(
		"    \"total_draws\": {},\n",
		dashboard.frame_pacing.total_draws
	));
	json.push_str(&format!(
		"    \"over_budget\": {},\n",
		dashboard.frame_pacing.over_budget
	));
	json.push_str(&format!(
		"    \"severe_jank\": {},\n",
		dashboard.frame_pacing.severe_jank
	));
	json.push_str(&format!("    \"cache_hits\": {},\n", dashboard.frame_pacing.cache_hits));
	json.push_str(&format!(
		"    \"cache_misses\": {},\n",
		dashboard.frame_pacing.cache_misses
	));
	json.push_str(&format!(
		"    \"recent_ms\": {}\n",
		json_numbers(&dashboard.frame_pacing.recent_ms)
	));
	json.push_str("  },\n");
	json.push_str("  \"hot_paths\": [\n");
	json.push_str(
		&dashboard
			.hot_paths
			.iter()
			.map(|summary| {
				format!(
					"    {{\"label\": {}, \"last_ms\": {}, \"avg_ms\": {}, \"p95_ms\": {}, \"max_ms\": {}, \"total_samples\": {}, \"over_warning\": {}, \"over_budget\": {}}}",
					json_string(summary.label),
					json_f32(summary.last_ms),
					json_f32(summary.avg_ms),
					json_f32(summary.p95_ms),
					json_f32(summary.max_ms),
					summary.total_samples,
					summary.over_warning,
					summary.over_budget,
				)
			})
			.collect::<Vec<_>>()
			.join(",\n"),
	);
	json.push_str("\n  ]\n");
	json.push('}');
	json
}

fn json_numbers(values: &[f32]) -> String {
	format!(
		"[{}]",
		values
			.iter()
			.map(|value| json_f32(*value))
			.collect::<Vec<_>>()
			.join(", ")
	)
}

fn json_f32(value: f32) -> String {
	format!("{value:.3}")
}

fn json_string(value: &str) -> String {
	let escaped = value
		.chars()
		.flat_map(|ch| match ch {
			'\\' => "\\\\".chars().collect::<Vec<_>>(),
			'"' => "\\\"".chars().collect(),
			'\n' => "\\n".chars().collect(),
			'\r' => "\\r".chars().collect(),
			'\t' => "\\t".chars().collect(),
			_ => [ch].into_iter().collect(),
		})
		.collect::<String>();

	format!("\"{escaped}\"")
}

#[cfg(test)]
mod tests {
	use {
		super::{DEFAULT_SAMPLE_FRAMES, DEFAULT_WARMUP_FRAMES, parse_args},
		crate::HeadlessScenario,
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

		assert_eq!(config.scenario, HeadlessScenario::TallInspect);
		assert_eq!(config.warmup_frames, 12);
		assert_eq!(config.sample_frames, 48);
	}

	#[test]
	fn parse_args_defaults_frame_counts() {
		let config = parse_args(&strings(&["--perf-scenario", "tall"]))
			.expect("args should parse")
			.expect("perf mode should be selected");

		assert_eq!(config.scenario, HeadlessScenario::Tall);
		assert_eq!(config.warmup_frames, DEFAULT_WARMUP_FRAMES);
		assert_eq!(config.sample_frames, DEFAULT_SAMPLE_FRAMES);
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
