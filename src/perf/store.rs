use {
	super::bridge::CanvasPerfSink,
	std::{
		collections::VecDeque,
		time::{Duration, Instant},
	},
};

pub(super) const HISTORY_LIMIT: usize = 180;
pub(super) const RECENT_LIMIT: usize = 8;
pub(super) const FRAME_BUDGET_MS: f32 = 16.7;
pub(super) const SEVERE_FRAME_MS: f32 = 33.3;
pub(super) const METRIC_WARNING_MS: f32 = 8.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub(super) enum MetricKind {
	EditorCommand,
	EditorApply,
	EditorWidthSync,
	SceneBuild,
	ResizeReflow,
	UiBuild,
	UiDraw,
	CanvasUpdate,
	CanvasStaticBuild,
	CanvasUnderlayDraw,
	CanvasOverlayDraw,
	CanvasDraw,
}

impl MetricKind {
	pub(super) const ALL: [Self; 12] = [
		Self::EditorCommand,
		Self::EditorApply,
		Self::EditorWidthSync,
		Self::SceneBuild,
		Self::ResizeReflow,
		Self::UiBuild,
		Self::UiDraw,
		Self::CanvasUpdate,
		Self::CanvasStaticBuild,
		Self::CanvasUnderlayDraw,
		Self::CanvasOverlayDraw,
		Self::CanvasDraw,
	];

	pub(super) fn label(self) -> &'static str {
		match self {
			Self::EditorCommand => "editor.command",
			Self::EditorApply => "editor.apply",
			Self::EditorWidthSync => "editor.width_sync",
			Self::SceneBuild => "scene.build",
			Self::ResizeReflow => "resize.reflow",
			Self::UiBuild => "ui.build",
			Self::UiDraw => "ui.draw",
			Self::CanvasUpdate => "canvas.update",
			Self::CanvasStaticBuild => "canvas.static",
			Self::CanvasUnderlayDraw => "canvas.underlay",
			Self::CanvasOverlayDraw => "canvas.overlay",
			Self::CanvasDraw => "canvas.draw",
		}
	}

	pub(super) fn index(self) -> usize {
		self as usize
	}
}

#[derive(Debug, Clone, Default)]
pub(super) struct MetricSeries {
	pub(super) window: VecDeque<f32>,
	pub(super) total_samples: u64,
	pub(super) over_warning: u64,
	pub(super) over_budget: u64,
}

impl MetricSeries {
	pub(super) fn record(&mut self, duration: Duration) {
		let sample_ms = duration.as_secs_f32() * 1000.0;
		push_bounded(&mut self.window, sample_ms, HISTORY_LIMIT);
		self.total_samples += 1;
		self.over_warning += u64::from(sample_ms >= METRIC_WARNING_MS);
		self.over_budget += u64::from(sample_ms >= FRAME_BUDGET_MS);
	}
}

#[derive(Debug, Clone, Default)]
pub(super) struct FrameSeries {
	pub(super) intervals_ms: VecDeque<f32>,
	last_draw_at: Option<Instant>,
	pub(super) total_draws: u64,
	pub(super) over_budget: u64,
	pub(super) severe_jank: u64,
}

impl FrameSeries {
	pub(super) fn record_draw(&mut self, at: Instant) {
		if let Some(previous) = self.last_draw_at.replace(at) {
			let interval_ms = (at - previous).as_secs_f32() * 1000.0;
			push_bounded(&mut self.intervals_ms, interval_ms, HISTORY_LIMIT);
			self.over_budget += u64::from(interval_ms >= FRAME_BUDGET_MS);
			self.severe_jank += u64::from(interval_ms >= SEVERE_FRAME_MS);
		}

		self.total_draws += 1;
	}
}

#[derive(Debug, Clone, Default)]
pub(super) struct CacheStats {
	pub(super) hits: u64,
	pub(super) misses: u64,
}

#[derive(Debug, Clone, Default)]
pub(super) struct PerfStore {
	pub(super) metrics: [MetricSeries; MetricKind::ALL.len()],
	pub(super) frames: FrameSeries,
	pub(super) cache: CacheStats,
}

impl PerfStore {
	pub(super) fn record_editor_apply(&mut self, duration: Duration) {
		self.record(MetricKind::EditorApply, duration);
	}

	pub(super) fn record_editor_command(&mut self, duration: Duration) {
		self.record(MetricKind::EditorCommand, duration);
	}

	pub(super) fn record_editor_width_sync(&mut self, duration: Duration) {
		self.record(MetricKind::EditorWidthSync, duration);
	}

	pub(super) fn record_scene_build(&mut self, duration: Duration) {
		self.record(MetricKind::SceneBuild, duration);
	}

	pub(super) fn record_resize_reflow(&mut self, duration: Duration) {
		self.record(MetricKind::ResizeReflow, duration);
	}

	pub(super) fn record_ui_build(&mut self, duration: Duration) {
		self.record(MetricKind::UiBuild, duration);
	}

	pub(super) fn record_ui_draw(&mut self, duration: Duration) {
		self.record(MetricKind::UiDraw, duration);
	}

	pub(super) fn flush_canvas_metrics(&mut self, sink: &CanvasPerfSink) {
		let pending = sink.drain();

		self.record_pending_durations(MetricKind::CanvasUpdate, pending.updates);
		self.record_pending_durations(MetricKind::CanvasUnderlayDraw, pending.underlay);
		self.record_pending_durations(MetricKind::CanvasOverlayDraw, pending.overlay);

		for sample in pending.draws {
			self.record(MetricKind::CanvasDraw, sample.total);
			if let Some(duration) = sample.static_build {
				self.record(MetricKind::CanvasStaticBuild, duration);
			}
			self.frames.record_draw(sample.drawn_at);

			if sample.cache_miss {
				self.cache.misses += 1;
			} else {
				self.cache.hits += 1;
			}
		}
	}

	fn record(&mut self, kind: MetricKind, duration: Duration) {
		self.metrics[kind.index()].record(duration);
	}

	fn record_pending_durations(&mut self, kind: MetricKind, durations: VecDeque<Duration>) {
		for duration in durations {
			self.record(kind, duration);
		}
	}
}

pub(super) fn average_ms(values: impl Iterator<Item = f32>) -> f32 {
	let mut total = 0.0;
	// The series already stores `f32`, so keep the accumulator in the same
	// domain instead of counting as `usize` and casting back at the end.
	let mut count = 0.0;

	for value in values {
		total += value;
		count += 1.0;
	}

	if count == 0.0 { 0.0 } else { total / count }
}

pub(super) fn percentile_ms(values: &VecDeque<f32>, percentile_percent: usize) -> f32 {
	if values.is_empty() {
		return 0.0;
	}

	let mut sorted = values.iter().copied().collect::<Vec<_>>();
	let index = ((sorted.len() - 1) * percentile_percent + 50) / 100;
	let (_, sample, _) = sorted.select_nth_unstable_by(index, f32::total_cmp);
	*sample
}

fn push_bounded<T>(items: &mut VecDeque<T>, value: T, limit: usize) {
	if items.len() == limit {
		items.pop_front();
	}

	items.push_back(value);
}

#[cfg(test)]
mod tests {
	use {
		super::{FRAME_BUDGET_MS, FrameSeries, MetricSeries, PerfStore, percentile_ms},
		crate::perf::CanvasPerfSink,
		std::time::{Duration, Instant},
	};

	#[test]
	fn metric_series_discards_evicted_spikes_from_the_window() {
		let mut series = MetricSeries::default();

		series.record(Duration::from_millis(99));
		for _ in 0..super::HISTORY_LIMIT {
			series.record(Duration::from_micros(500));
		}

		assert!(series.window.iter().all(|sample| *sample < 2.0));
		assert_eq!(series.total_samples, (super::HISTORY_LIMIT + 1) as u64);
		assert_eq!(series.over_budget, 1);
	}

	#[test]
	fn percentile_returns_zero_for_empty_series() {
		assert!((percentile_ms(&std::collections::VecDeque::new(), 95) - 0.0).abs() <= 0.001);
	}

	#[test]
	fn frame_series_tracks_over_budget_intervals() {
		let mut frames = FrameSeries::default();
		let started = Instant::now();

		frames.record_draw(started);
		frames.record_draw(started + Duration::from_secs_f32((FRAME_BUDGET_MS + 2.0) / 1000.0));

		assert_eq!(frames.total_draws, 2);
		assert_eq!(frames.over_budget, 1);
	}

	#[test]
	fn store_flushes_bridge_samples_into_metrics() {
		let mut store = PerfStore::default();
		let sink = CanvasPerfSink::default();

		sink.record_canvas_update(Duration::from_millis(3));
		sink.record_canvas_underlay(Duration::from_millis(1));
		sink.record_canvas_overlay(Duration::from_millis(1));
		sink.record_canvas_draw(Duration::from_millis(5), Some(Duration::from_millis(2)), false);
		store.flush_canvas_metrics(&sink);

		assert_eq!(store.metrics[super::MetricKind::CanvasUpdate.index()].total_samples, 1);
		assert_eq!(
			store.metrics[super::MetricKind::CanvasUnderlayDraw.index()].total_samples,
			1
		);
		assert_eq!(
			store.metrics[super::MetricKind::CanvasOverlayDraw.index()].total_samples,
			1
		);
		assert_eq!(store.metrics[super::MetricKind::CanvasDraw.index()].total_samples, 1);
		assert_eq!(store.cache.hits, 1);
	}
}
