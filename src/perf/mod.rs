mod bridge;
mod report;
mod store;

use {
	self::{bridge::CanvasPerfSink as Sink, report::build_dashboard, store::PerfStore},
	crate::{editor::EditorMode, scene::DocumentLayout},
	std::{hash::Hash, time::Duration},
};
pub(crate) use {
	bridge::CanvasPerfSink,
	report::{PerfDashboard, PerfGraphSeries, PerfMetricSummary, PerfRecentActivity, unavailable_dashboard},
};

#[derive(Debug, Default)]
pub(crate) struct PerfMonitor {
	store: PerfStore,
	sink: Sink,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct PerfSnapshotKey {
	metric_totals: [u64; store::MetricKind::ALL.len()],
	total_draws: u64,
	cache_hits: u64,
	cache_misses: u64,
}

impl PerfMonitor {
	pub(crate) fn sink(&self) -> CanvasPerfSink {
		Clone::clone(&self.sink)
	}

	pub(crate) fn key(&self) -> PerfSnapshotKey {
		PerfSnapshotKey {
			metric_totals: store::MetricKind::ALL.map(|kind| self.store.metrics[kind.index()].total_samples),
			total_draws: self.store.frames.total_draws,
			cache_hits: self.store.cache.hits,
			cache_misses: self.store.cache.misses,
		}
	}

	pub(crate) fn record_editor_apply(&mut self, duration: Duration) {
		self.store.record(store::MetricKind::EditorApply, duration);
	}

	pub(crate) fn record_editor_command(&mut self, duration: Duration) {
		self.store.record(store::MetricKind::EditorCommand, duration);
	}

	pub(crate) fn record_editor_width_sync(&mut self, duration: Duration) {
		self.store.record(store::MetricKind::EditorWidthSync, duration);
	}

	pub(crate) fn record_scene_build(&mut self, duration: Duration) {
		self.store.record(store::MetricKind::SceneBuild, duration);
	}

	pub(crate) fn record_resize_reflow(&mut self, duration: Duration) {
		self.store.record(store::MetricKind::ResizeReflow, duration);
	}

	pub(crate) fn record_ui_build(&mut self, duration: Duration) {
		self.store.record(store::MetricKind::UiBuild, duration);
	}

	pub(crate) fn record_ui_draw(&mut self, duration: Duration) {
		self.store.record(store::MetricKind::UiDraw, duration);
	}

	pub(crate) fn flush_canvas_metrics(&mut self) {
		self.store.flush_canvas_metrics(&self.sink);
	}

	pub(crate) fn dashboard(
		&self, layout: &DocumentLayout, editor_mode: EditorMode, editor_bytes: usize,
	) -> PerfDashboard {
		build_dashboard(&self.store, layout, editor_mode, editor_bytes)
	}

	#[cfg(test)]
	pub(crate) fn metric_total_samples(&self, label: &str) -> u64 {
		store::MetricKind::ALL
			.into_iter()
			.find(|kind| kind.label() == label)
			.map_or(0, |kind| self.store.metrics[kind.index()].total_samples)
	}
}
