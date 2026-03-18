use {
	super::store::{
		FRAME_BUDGET_MS, METRIC_WARNING_MS, MetricKind, PerfStore, RECENT_LIMIT, SEVERE_FRAME_MS, average_ms,
		percentile_ms,
	},
	crate::{editor::EditorMode, scene::DocumentLayout},
	std::{fmt::Write as _, sync::Arc},
};

const GRAPH_METRICS: [MetricKind; 12] = [
	MetricKind::EditorWidthSync,
	MetricKind::UiBuild,
	MetricKind::UiDraw,
	MetricKind::CanvasDraw,
	MetricKind::CanvasStaticBuild,
	MetricKind::CanvasUnderlayDraw,
	MetricKind::CanvasOverlayDraw,
	MetricKind::CanvasUpdate,
	MetricKind::ResizeReflow,
	MetricKind::EditorCommand,
	MetricKind::SceneBuild,
	MetricKind::EditorApply,
];
const RECENT_ACTIVITY_METRICS: [MetricKind; 9] = [
	MetricKind::EditorWidthSync,
	MetricKind::ResizeReflow,
	MetricKind::UiBuild,
	MetricKind::UiDraw,
	MetricKind::CanvasUpdate,
	MetricKind::CanvasStaticBuild,
	MetricKind::CanvasUnderlayDraw,
	MetricKind::CanvasOverlayDraw,
	MetricKind::CanvasDraw,
];

#[derive(Debug, Clone)]
pub struct PerfGraphSeries {
	pub title: &'static str,
	pub samples_ms: Arc<[f32]>,
	pub ceiling_ms: f32,
	pub latest_ms: f32,
	pub avg_ms: f32,
	pub p95_ms: f32,
	pub warning_ms: Option<f32>,
	pub severe_ms: Option<f32>,
}

#[derive(Debug, Clone)]
pub struct PerfOverview {
	pub editor_mode: EditorMode,
	pub editor_bytes: usize,
	pub editor_chars: usize,
	pub line_count: usize,
	pub run_count: usize,
	pub glyph_count: usize,
	pub cluster_count: usize,
	pub font_count: usize,
	pub warning_count: usize,
	pub scene_width: f32,
	pub scene_height: f32,
	pub layout_width: f32,
}

impl PerfOverview {
	pub fn text(&self) -> String {
		format!(
			"editor mode   {}\nbytes/chars   {} / {}\nlines         {}\nruns/glyphs   {} / {}\nclusters      {}\nfonts seen    {}\nwarnings      {}\nscene size    {:.1} x {:.1}\nlayout width  {:.1}",
			self.editor_mode,
			self.editor_bytes,
			self.editor_chars,
			self.line_count,
			self.run_count,
			self.glyph_count,
			self.cluster_count,
			self.font_count,
			self.warning_count,
			self.scene_width,
			self.scene_height,
			self.layout_width,
		)
	}
}

#[derive(Debug, Clone)]
pub struct PerfMetricSummary {
	pub label: &'static str,
	pub last_ms: f32,
	pub avg_ms: f32,
	pub p95_ms: f32,
	pub max_ms: f32,
	pub total_samples: u64,
	pub over_warning: u64,
	pub over_budget: u64,
}

impl PerfMetricSummary {
	pub fn text(&self) -> String {
		format!(
			"{:<14} last {:>5.2} ms  avg {:>5.2}  p95 {:>5.2}  max {:>5.2}  n {:>4}  >8 {:>4}  >16 {:>4}",
			self.label,
			self.last_ms,
			self.avg_ms,
			self.p95_ms,
			self.max_ms,
			self.total_samples,
			self.over_warning,
			self.over_budget,
		)
	}
}

#[derive(Debug, Clone)]
pub struct PerfRecentActivity {
	pub label: &'static str,
	pub recent_ms: Arc<[f32]>,
}

impl PerfRecentActivity {
	pub fn text(&self) -> String {
		if self.recent_ms.is_empty() {
			return format!("{:<14} no samples", self.label);
		}

		let mut text = format!("{:<14} ", self.label);
		for (index, value) in self.recent_ms.iter().enumerate() {
			if index > 0 {
				text.push_str("  ");
			}
			let _ = write!(text, "{value:>5.2}");
		}
		text
	}
}

#[derive(Debug, Clone)]
pub struct PerfFramePacingSummary {
	pub fps: f32,
	pub last_ms: f32,
	pub avg_ms: f32,
	pub max_ms: f32,
	pub total_draws: u64,
	pub over_budget: u64,
	pub severe_jank: u64,
	pub cache_hits: u64,
	pub cache_misses: u64,
	pub recent_ms: Arc<[f32]>,
}

impl PerfFramePacingSummary {
	pub fn text(&self) -> String {
		let total = self.cache_hits + self.cache_misses;
		let miss_rate_tenths = (self.cache_misses.saturating_mul(1000) + total / 2)
			.checked_div(total)
			.unwrap_or_default();
		let miss_rate_whole = miss_rate_tenths / 10;
		let miss_rate_fraction = miss_rate_tenths % 10;

		format!(
			"canvas fps    {:>5.1}\nframe last    {:>5.2} ms\nframe avg     {:>5.2} ms\nframe max     {:>5.2} ms\ndraw calls    {:>5}\n>16.7 ms      {:>5}\n>33.3 ms      {:>5}\ncache hits    {:>5}\ncache misses  {:>5}\nmiss rate     {:>4}.{} %",
			self.fps,
			self.last_ms,
			self.avg_ms,
			self.max_ms,
			self.total_draws,
			self.over_budget,
			self.severe_jank,
			self.cache_hits,
			self.cache_misses,
			miss_rate_whole,
			miss_rate_fraction,
		)
	}

	pub fn recent_activity(&self) -> PerfRecentActivity {
		PerfRecentActivity {
			label: "frame delta",
			recent_ms: self.recent_ms.clone(),
		}
	}
}

#[derive(Debug, Clone)]
pub struct PerfDashboard {
	pub overview: PerfOverview,
	pub hot_paths: Vec<PerfMetricSummary>,
	pub recent_activity: Vec<PerfRecentActivity>,
	pub frame_pacing: PerfFramePacingSummary,
	pub graphs: Vec<PerfGraphSeries>,
}

pub fn unavailable_dashboard(editor_mode: EditorMode, editor_bytes: usize, layout_width: f32) -> PerfDashboard {
	PerfDashboard {
		overview: PerfOverview {
			editor_mode,
			editor_bytes,
			editor_chars: 0,
			line_count: 0,
			run_count: 0,
			glyph_count: 0,
			cluster_count: 0,
			font_count: 0,
			warning_count: 0,
			scene_width: 0.0,
			scene_height: 0.0,
			layout_width,
		},
		hot_paths: Vec::new(),
		recent_activity: vec![PerfRecentActivity {
			label: "scene",
			recent_ms: Arc::from([]),
		}],
		frame_pacing: PerfFramePacingSummary {
			fps: 0.0,
			last_ms: 0.0,
			avg_ms: 0.0,
			max_ms: 0.0,
			total_draws: 0,
			over_budget: 0,
			severe_jank: 0,
			cache_hits: 0,
			cache_misses: 0,
			recent_ms: Arc::from([]),
		},
		graphs: vec![PerfGraphSeries {
			title: "scene",
			samples_ms: Arc::from([]),
			ceiling_ms: 1.0,
			latest_ms: 0.0,
			avg_ms: 0.0,
			p95_ms: 0.0,
			warning_ms: None,
			severe_ms: None,
		}],
	}
}

pub(super) fn build_dashboard(
	store: &PerfStore, layout: &DocumentLayout, editor_mode: EditorMode, editor_bytes: usize,
) -> PerfDashboard {
	// Summaries back both the table and the graphs; compute them once so a
	// dashboard rebuild does not rescan the same metric windows twice.
	let hot_paths = MetricKind::ALL
		.into_iter()
		.map(|kind| metric_summary(store, kind))
		.collect::<Vec<_>>();
	let frame_pacing = frame_pacing_summary(store);
	let recent_activity = RECENT_ACTIVITY_METRICS
		.into_iter()
		.map(|kind| recent_metric_activity(store, kind))
		.chain(std::iter::once(frame_pacing.recent_activity()))
		.collect();
	let graphs = graphs(store, &frame_pacing, &hot_paths);

	PerfDashboard {
		overview: PerfOverview {
			editor_mode,
			editor_bytes,
			editor_chars: layout.text.chars().count(),
			line_count: layout.text.lines().count().max(1),
			run_count: layout.runs.len(),
			glyph_count: layout.glyph_count,
			cluster_count: layout.cluster_count,
			font_count: layout.font_count,
			warning_count: layout.warnings.len(),
			scene_width: layout.measured_width,
			scene_height: layout.measured_height,
			layout_width: layout.max_width,
		},
		hot_paths,
		recent_activity,
		graphs,
		frame_pacing,
	}
}

fn metric_summary(store: &PerfStore, kind: MetricKind) -> PerfMetricSummary {
	let series = &store.metrics[kind.index()];
	PerfMetricSummary {
		label: kind.label(),
		last_ms: series.window.back().copied().unwrap_or_default(),
		avg_ms: average_ms(series.window.iter().copied()),
		p95_ms: percentile_ms(&series.window, 95),
		max_ms: series.window.iter().copied().fold(0.0, f32::max),
		total_samples: series.total_samples,
		over_warning: series.over_warning,
		over_budget: series.over_budget,
	}
}

fn recent_metric_activity(store: &PerfStore, kind: MetricKind) -> PerfRecentActivity {
	PerfRecentActivity {
		label: kind.label(),
		recent_ms: recent_values(&store.metrics[kind.index()].window),
	}
}

fn frame_pacing_summary(store: &PerfStore) -> PerfFramePacingSummary {
	let last_ms = store.frames.intervals_ms.back().copied().unwrap_or_default();
	let avg_ms = average_ms(store.frames.intervals_ms.iter().copied());
	let max_ms = store.frames.intervals_ms.iter().copied().fold(0.0, f32::max);
	let fps = if avg_ms > 0.0 { 1000.0 / avg_ms } else { 0.0 };

	PerfFramePacingSummary {
		fps,
		last_ms,
		avg_ms,
		max_ms,
		total_draws: store.frames.total_draws,
		over_budget: store.frames.over_budget,
		severe_jank: store.frames.severe_jank,
		cache_hits: store.cache.hits,
		cache_misses: store.cache.misses,
		recent_ms: recent_values(&store.frames.intervals_ms),
	}
}

fn graphs(
	store: &PerfStore, frame_pacing: &PerfFramePacingSummary, hot_paths: &[PerfMetricSummary],
) -> Vec<PerfGraphSeries> {
	let frame_p95 = percentile_ms(&store.frames.intervals_ms, 95);
	let frame_graph = PerfGraphSeries {
		title: "frame delta",
		samples_ms: store.frames.intervals_ms.iter().copied().collect(),
		ceiling_ms: graph_ceiling(frame_pacing.max_ms.max(frame_p95).max(SEVERE_FRAME_MS), false),
		latest_ms: frame_pacing.last_ms,
		avg_ms: frame_pacing.avg_ms,
		p95_ms: frame_p95,
		warning_ms: Some(FRAME_BUDGET_MS),
		severe_ms: Some(SEVERE_FRAME_MS),
	};

	std::iter::once(frame_graph)
		.chain(GRAPH_METRICS.into_iter().map(|kind| {
			let summary = &hot_paths[kind.index()];
			PerfGraphSeries {
				title: kind.label(),
				samples_ms: store.metrics[kind.index()].window.iter().copied().collect(),
				ceiling_ms: graph_ceiling(summary.max_ms.max(summary.p95_ms), true),
				latest_ms: summary.last_ms,
				avg_ms: summary.avg_ms,
				p95_ms: summary.p95_ms,
				warning_ms: Some(METRIC_WARNING_MS),
				severe_ms: Some(FRAME_BUDGET_MS),
			}
		}))
		.collect()
}

pub(super) fn graph_ceiling(max_sample_ms: f32, keep_low_range: bool) -> f32 {
	let ladder = if keep_low_range {
		[
			0.05, 0.1, 0.25, 0.5, 1.0, 2.0, 4.0, 8.0, 16.7, 33.3, 50.0, 75.0, 100.0, 150.0, 250.0, 500.0,
		]
	} else {
		[
			0.1, 0.25, 0.5, 1.0, 2.0, 4.0, 8.0, 16.7, 33.3, 50.0, 75.0, 100.0, 150.0, 250.0, 500.0, 500.0,
		]
	};

	ladder
		.into_iter()
		.find(|ceiling| *ceiling >= max_sample_ms.max(0.01) * 1.15)
		.unwrap_or_else(|| (max_sample_ms.max(0.1) * 1.2).ceil())
}

fn recent_values(values: &std::collections::VecDeque<f32>) -> Arc<[f32]> {
	values.iter().rev().take(RECENT_LIMIT).copied().collect()
}

#[cfg(test)]
mod tests {
	use {
		super::{build_dashboard, graph_ceiling},
		crate::{
			editor::EditorMode,
			perf::{CanvasPerfSink, store::PerfStore},
			scene::DocumentLayout,
			types::{FontChoice, ShapingChoice, WrapChoice},
		},
		glorp_editor::{build_buffer, make_font_system, resolve_font_names_from_buffer, scene_config},
		std::time::Duration,
	};

	fn scene() -> DocumentLayout {
		let mut font_system = make_font_system();
		let config = scene_config(
			FontChoice::Monospace,
			ShapingChoice::Auto,
			WrapChoice::Word,
			16.0,
			20.0,
			300.0,
		);
		let text = "abc\ndef";
		let buffer = build_buffer(&mut font_system, text, config);
		let font_names = resolve_font_names_from_buffer(&font_system, &buffer);
		DocumentLayout::build(text, &buffer, config, font_names.as_ref())
	}

	#[test]
	fn dashboard_derives_graphs_and_recent_activity() {
		let mut store = PerfStore::default();
		let sink = CanvasPerfSink::default();
		sink.record_canvas_update(Duration::from_millis(2));
		sink.record_canvas_underlay(Duration::from_millis(1));
		sink.record_canvas_overlay(Duration::from_millis(1));
		sink.record_canvas_draw(Duration::from_millis(5), Some(Duration::from_millis(3)), false);
		store.flush_canvas_metrics(&sink);

		let dashboard = build_dashboard(&store, &scene(), EditorMode::Normal, 7);

		assert!(!dashboard.graphs.is_empty());
		assert_eq!(dashboard.recent_activity.len(), 10);
		assert_eq!(dashboard.overview.editor_bytes, 7);
	}

	#[test]
	fn graph_ceiling_keeps_small_ranges_stable() {
		assert!((graph_ceiling(7.0, true) - 16.7).abs() <= 0.001);
		assert!(graph_ceiling(40.0, false) >= 50.0);
	}

	#[test]
	fn recent_activity_formatting_is_stable_after_split() {
		let mut store = PerfStore::default();
		let sink = CanvasPerfSink::default();
		sink.record_canvas_update(Duration::from_millis(1));
		sink.record_canvas_update(Duration::from_millis(2));
		store.flush_canvas_metrics(&sink);

		let dashboard = build_dashboard(&store, &scene(), EditorMode::Insert, 7);
		let update_line = dashboard
			.recent_activity
			.iter()
			.find(|entry| entry.label == "canvas.update")
			.expect("canvas.update recent activity should exist");

		assert_eq!(update_line.text(), "canvas.update   2.00   1.00");
	}
}
