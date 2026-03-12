use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::editor::EditorMode;
use crate::scene::LayoutScene;

const HISTORY_LIMIT: usize = 180;
const RECENT_LIMIT: usize = 8;
const PENDING_LIMIT: usize = 512;
const FRAME_BUDGET_MS: f32 = 16.7;
const SEVERE_FRAME_MS: f32 = 33.3;
const METRIC_WARNING_MS: f32 = 8.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MetricKind {
	EditorCommand,
	EditorApply,
	SceneBuild,
	ResizeReflow,
	CanvasUpdate,
	CanvasStaticBuild,
	CanvasOverlayDraw,
	CanvasDraw,
}

impl MetricKind {
	const ALL: [Self; 8] = [
		Self::EditorCommand,
		Self::EditorApply,
		Self::SceneBuild,
		Self::ResizeReflow,
		Self::CanvasUpdate,
		Self::CanvasStaticBuild,
		Self::CanvasOverlayDraw,
		Self::CanvasDraw,
	];

	fn label(self) -> &'static str {
		match self {
			Self::EditorCommand => "editor.command",
			Self::EditorApply => "editor.apply",
			Self::SceneBuild => "scene.build",
			Self::ResizeReflow => "resize.reflow",
			Self::CanvasUpdate => "canvas.update",
			Self::CanvasStaticBuild => "canvas.static",
			Self::CanvasOverlayDraw => "canvas.overlay",
			Self::CanvasDraw => "canvas.draw",
		}
	}

	fn index(self) -> usize {
		match self {
			Self::EditorCommand => 0,
			Self::EditorApply => 1,
			Self::SceneBuild => 2,
			Self::ResizeReflow => 3,
			Self::CanvasUpdate => 4,
			Self::CanvasStaticBuild => 5,
			Self::CanvasOverlayDraw => 6,
			Self::CanvasDraw => 7,
		}
	}
}

#[derive(Debug, Clone, Default)]
struct MetricSeries {
	window: VecDeque<f32>,
	total_samples: u64,
	over_8ms: u64,
	over_16ms: u64,
}

impl MetricSeries {
	fn record(&mut self, duration: Duration) {
		let sample_ms = duration.as_secs_f32() * 1000.0;
		push_bounded(&mut self.window, sample_ms, HISTORY_LIMIT);
		self.total_samples += 1;
		self.over_8ms += u64::from(sample_ms >= 8.0);
		self.over_16ms += u64::from(sample_ms >= FRAME_BUDGET_MS);
	}

	fn summary(&self, label: &'static str) -> MetricSummary<'_> {
		let last_ms = self.window.back().copied().unwrap_or_default();
		let avg_ms = average_ms(self.window.iter().copied());
		let max_ms = self.window.iter().copied().fold(0.0, f32::max);
		let p95_ms = percentile_ms(&self.window, 0.95);

		MetricSummary {
			label,
			last_ms,
			avg_ms,
			p95_ms,
			max_ms,
			total_samples: self.total_samples,
			over_8ms: self.over_8ms,
			over_16ms: self.over_16ms,
			recent: &self.window,
		}
	}
}

#[derive(Debug, Default)]
struct FrameSeries {
	intervals_ms: VecDeque<f32>,
	last_draw_at: Option<Instant>,
	total_draws: u64,
	over_budget: u64,
	severe_jank: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct PerfGraphSeries {
	pub(crate) title: &'static str,
	pub(crate) samples_ms: Vec<f32>,
	pub(crate) ceiling_ms: f32,
	pub(crate) latest_ms: f32,
	pub(crate) avg_ms: f32,
	pub(crate) p95_ms: f32,
	pub(crate) warning_ms: Option<f32>,
	pub(crate) severe_ms: Option<f32>,
}

impl FrameSeries {
	fn record_draw(&mut self, at: Instant) {
		if let Some(previous) = self.last_draw_at.replace(at) {
			let interval_ms = (at - previous).as_secs_f32() * 1000.0;
			push_bounded(&mut self.intervals_ms, interval_ms, HISTORY_LIMIT);
			self.over_budget += u64::from(interval_ms >= FRAME_BUDGET_MS);
			self.severe_jank += u64::from(interval_ms >= SEVERE_FRAME_MS);
		}

		self.total_draws += 1;
	}

	fn summary(&self) -> FrameSummary<'_> {
		let last_ms = self.intervals_ms.back().copied().unwrap_or_default();
		let avg_ms = average_ms(self.intervals_ms.iter().copied());
		let max_ms = self.intervals_ms.iter().copied().fold(0.0, f32::max);
		let fps = if avg_ms > 0.0 { 1000.0 / avg_ms } else { 0.0 };

		FrameSummary {
			last_ms,
			avg_ms,
			max_ms,
			fps,
			total_draws: self.total_draws,
			over_budget: self.over_budget,
			severe_jank: self.severe_jank,
			recent: &self.intervals_ms,
		}
	}
}

#[derive(Debug, Default)]
struct PendingSamples {
	canvas_update: VecDeque<Duration>,
	canvas_draw: VecDeque<CanvasDrawSample>,
}

#[derive(Debug, Clone, Copy)]
struct CanvasDrawSample {
	total: Duration,
	static_build: Option<Duration>,
	overlay: Duration,
	drawn_at: Instant,
	cache_miss: bool,
}

#[derive(Debug, Default)]
struct CacheStats {
	hits: u64,
	misses: u64,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct PerfBridge {
	pending: Arc<Mutex<PendingSamples>>,
}

impl PerfBridge {
	pub(crate) fn record_canvas_update(&self, duration: Duration) {
		let Ok(mut pending) = self.pending.lock() else {
			return;
		};

		push_bounded(&mut pending.canvas_update, duration, PENDING_LIMIT);
	}

	pub(crate) fn record_canvas_draw(
		&self, total: Duration, static_build: Option<Duration>, overlay: Duration, cache_miss: bool,
	) {
		let Ok(mut pending) = self.pending.lock() else {
			return;
		};

		push_bounded(
			&mut pending.canvas_draw,
			CanvasDrawSample {
				total,
				static_build,
				overlay,
				drawn_at: Instant::now(),
				cache_miss,
			},
			PENDING_LIMIT,
		);
	}

	fn drain(&self) -> PendingSamples {
		let Ok(mut pending) = self.pending.lock() else {
			return PendingSamples::default();
		};

		PendingSamples {
			canvas_update: pending.canvas_update.drain(..).collect(),
			canvas_draw: pending.canvas_draw.drain(..).collect(),
		}
	}
}

#[derive(Debug, Default)]
pub(crate) struct PerfMonitor {
	metrics: [MetricSeries; MetricKind::ALL.len()],
	frames: FrameSeries,
	cache: CacheStats,
	bridge: PerfBridge,
}

impl PerfMonitor {
	pub(crate) fn bridge(&self) -> PerfBridge {
		self.bridge.clone()
	}

	pub(crate) fn record_editor_apply(&mut self, duration: Duration) {
		self.record(MetricKind::EditorApply, duration);
	}

	pub(crate) fn record_editor_command(&mut self, duration: Duration) {
		self.record(MetricKind::EditorCommand, duration);
	}

	pub(crate) fn record_scene_build(&mut self, duration: Duration) {
		self.record(MetricKind::SceneBuild, duration);
	}

	pub(crate) fn record_resize_reflow(&mut self, duration: Duration) {
		self.record(MetricKind::ResizeReflow, duration);
	}

	pub(crate) fn flush_canvas_metrics(&mut self) {
		let pending = self.bridge.drain();

		for duration in pending.canvas_update {
			self.record(MetricKind::CanvasUpdate, duration);
		}

		for sample in pending.canvas_draw {
			self.record(MetricKind::CanvasDraw, sample.total);
			if let Some(duration) = sample.static_build {
				self.record(MetricKind::CanvasStaticBuild, duration);
			}
			self.record(MetricKind::CanvasOverlayDraw, sample.overlay);
			self.frames.record_draw(sample.drawn_at);

			if sample.cache_miss {
				self.cache.misses += 1;
			} else {
				self.cache.hits += 1;
			}
		}
	}

	pub(crate) fn overview_text(&self, scene: &LayoutScene, editor_mode: EditorMode, editor_bytes: usize) -> String {
		format!(
			"editor mode   {editor_mode}\nbytes/chars   {editor_bytes} / {}\nlines         {}\nruns/glyphs   {} / {}\nclusters      {}\nfonts seen    {}\nwarnings      {}\nscene size    {:.1} x {:.1}\nlayout width  {:.1}",
			scene.text.chars().count(),
			scene.text.lines().count().max(1),
			scene.runs.len(),
			scene.glyph_count,
			scene.clusters().len(),
			scene.font_count,
			scene.warnings.len(),
			scene.measured_width,
			scene.measured_height,
			scene.max_width,
		)
	}

	pub(crate) fn hot_paths_text(&self) -> String {
		MetricKind::ALL
			.into_iter()
			.map(|kind| self.metrics[kind.index()].summary(kind.label()).report())
			.collect::<Vec<_>>()
			.join("\n")
	}

	pub(crate) fn frame_pacing_text(&self) -> String {
		let hits = self.cache.hits;
		let misses = self.cache.misses;
		let total = hits + misses;
		let miss_rate = if total > 0 {
			(misses as f32 / total as f32) * 100.0
		} else {
			0.0
		};

		format!(
			"{}\ncache hits    {:>5}\ncache misses  {:>5}\nmiss rate     {:>5.1} %",
			self.frames.summary().report(),
			hits,
			misses,
			miss_rate,
		)
	}

	pub(crate) fn recent_activity_text(&self) -> String {
		let update = self.metrics[MetricKind::CanvasUpdate.index()]
			.summary(MetricKind::CanvasUpdate.label())
			.recent_report();
		let resize = self.metrics[MetricKind::ResizeReflow.index()]
			.summary(MetricKind::ResizeReflow.label())
			.recent_report();
		let draw = self.metrics[MetricKind::CanvasDraw.index()]
			.summary(MetricKind::CanvasDraw.label())
			.recent_report();
		let static_build = self.metrics[MetricKind::CanvasStaticBuild.index()]
			.summary(MetricKind::CanvasStaticBuild.label())
			.recent_report();
		let overlay = self.metrics[MetricKind::CanvasOverlayDraw.index()]
			.summary(MetricKind::CanvasOverlayDraw.label())
			.recent_report();
		let frames = self.frames.summary().recent_report();

		format!("{resize}\n{update}\n{static_build}\n{overlay}\n{draw}\n{frames}")
	}

	pub(crate) fn graphs(&self) -> Vec<PerfGraphSeries> {
		let frame_summary = self.frames.summary();
		let mut graphs = vec![PerfGraphSeries {
			title: "frame delta",
			samples_ms: self.frames.intervals_ms.iter().copied().collect(),
			ceiling_ms: graph_ceiling(
				frame_summary.max_ms.max(frame_summary.p95_hint()).max(SEVERE_FRAME_MS),
				false,
			),
			latest_ms: frame_summary.last_ms,
			avg_ms: frame_summary.avg_ms,
			p95_ms: percentile_ms(&self.frames.intervals_ms, 0.95),
			warning_ms: Some(FRAME_BUDGET_MS),
			severe_ms: Some(SEVERE_FRAME_MS),
		}];

		for kind in [
			MetricKind::CanvasDraw,
			MetricKind::CanvasStaticBuild,
			MetricKind::CanvasOverlayDraw,
			MetricKind::CanvasUpdate,
			MetricKind::ResizeReflow,
			MetricKind::EditorCommand,
			MetricKind::SceneBuild,
			MetricKind::EditorApply,
		] {
			let summary = self.metrics[kind.index()].summary(kind.label());
			graphs.push(PerfGraphSeries {
				title: kind.label(),
				samples_ms: self.metrics[kind.index()].window.iter().copied().collect(),
				ceiling_ms: graph_ceiling(summary.max_ms.max(summary.p95_ms), true),
				latest_ms: summary.last_ms,
				avg_ms: summary.avg_ms,
				p95_ms: summary.p95_ms,
				warning_ms: Some(METRIC_WARNING_MS),
				severe_ms: Some(FRAME_BUDGET_MS),
			});
		}

		graphs
	}

	fn record(&mut self, kind: MetricKind, duration: Duration) {
		self.metrics[kind.index()].record(duration);
	}
}

#[derive(Debug)]
struct MetricSummary<'a> {
	label: &'static str,
	last_ms: f32,
	avg_ms: f32,
	p95_ms: f32,
	max_ms: f32,
	total_samples: u64,
	over_8ms: u64,
	over_16ms: u64,
	recent: &'a VecDeque<f32>,
}

impl MetricSummary<'_> {
	fn report(&self) -> String {
		format!(
			"{:<14} last {:>5.2} ms  avg {:>5.2}  p95 {:>5.2}  max {:>5.2}  n {:>4}  >8 {:>4}  >16 {:>4}",
			self.label,
			self.last_ms,
			self.avg_ms,
			self.p95_ms,
			self.max_ms,
			self.total_samples,
			self.over_8ms,
			self.over_16ms,
		)
	}

	fn recent_report(&self) -> String {
		format!("{:<14} {}", self.label, recent_values(self.recent))
	}
}

#[derive(Debug)]
struct FrameSummary<'a> {
	last_ms: f32,
	avg_ms: f32,
	max_ms: f32,
	fps: f32,
	total_draws: u64,
	over_budget: u64,
	severe_jank: u64,
	recent: &'a VecDeque<f32>,
}

impl FrameSummary<'_> {
	fn p95_hint(&self) -> f32 {
		percentile_ms(self.recent, 0.95)
	}

	fn report(&self) -> String {
		format!(
			"canvas fps    {:>5.1}\nframe last    {:>5.2} ms\nframe avg     {:>5.2} ms\nframe max     {:>5.2} ms\ndraw calls    {:>5}\n>16.7 ms      {:>5}\n>33.3 ms      {:>5}",
			self.fps, self.last_ms, self.avg_ms, self.max_ms, self.total_draws, self.over_budget, self.severe_jank,
		)
	}

	fn recent_report(&self) -> String {
		format!("{:<14} {}", "frame delta", recent_values(self.recent))
	}
}

fn graph_ceiling(max_sample_ms: f32, keep_low_range: bool) -> f32 {
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

fn push_bounded<T>(items: &mut VecDeque<T>, value: T, limit: usize) {
	if items.len() == limit {
		let _ = items.pop_front();
	}

	items.push_back(value);
}

fn average_ms(values: impl Iterator<Item = f32>) -> f32 {
	let mut total = 0.0;
	let mut count = 0.0;

	for value in values {
		total += value;
		count += 1.0;
	}

	if count > 0.0 { total / count } else { 0.0 }
}

fn percentile_ms(values: &VecDeque<f32>, percentile: f32) -> f32 {
	if values.is_empty() {
		return 0.0;
	}

	let mut sorted = values.iter().copied().collect::<Vec<_>>();
	sorted.sort_by(f32::total_cmp);

	let index = ((sorted.len() - 1) as f32 * percentile).round() as usize;
	sorted[index]
}

fn recent_values(values: &VecDeque<f32>) -> String {
	if values.is_empty() {
		return "no samples".to_string();
	}

	values
		.iter()
		.rev()
		.take(RECENT_LIMIT)
		.map(|value| format!("{value:>5.2}"))
		.collect::<Vec<_>>()
		.join("  ")
}

#[cfg(test)]
mod tests {
	use std::time::Duration;

	use super::{FrameSeries, MetricSeries, percentile_ms};

	#[test]
	fn metric_series_discards_evicted_spikes_from_the_window() {
		let mut series = MetricSeries::default();

		series.record(Duration::from_millis(99));

		for _ in 0..220 {
			series.record(Duration::from_millis(2));
		}

		let summary = series.summary("scene.build");

		assert_eq!(summary.last_ms, 2.0);
		assert_eq!(summary.max_ms, 2.0);
		assert_eq!(summary.p95_ms, 2.0);
		assert_eq!(summary.total_samples, 221);
		assert_eq!(summary.over_16ms, 1);
	}

	#[test]
	fn frame_series_tracks_budget_and_severe_jank_separately() {
		let mut frames = FrameSeries::default();
		let start = std::time::Instant::now();

		frames.record_draw(start);
		frames.record_draw(start + Duration::from_millis(14));
		frames.record_draw(start + Duration::from_millis(36));
		frames.record_draw(start + Duration::from_millis(96));

		let summary = frames.summary();

		assert_eq!(summary.total_draws, 4);
		assert_eq!(summary.over_budget, 2);
		assert_eq!(summary.severe_jank, 1);
		assert!(summary.avg_ms > 20.0);
	}

	#[test]
	fn percentile_uses_the_current_sample_window() {
		let values = [1.0, 2.0, 3.0, 20.0].into_iter().collect();

		assert_eq!(percentile_ms(&values, 0.95), 20.0);
		assert_eq!(percentile_ms(&values, 0.50), 3.0);
	}
}
