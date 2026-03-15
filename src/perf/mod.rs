mod bridge;
mod report;
mod store;

use {
	self::{bridge::CanvasPerfSink as Sink, report::build_dashboard, store::PerfStore},
	crate::{editor::EditorMode, scene::LayoutScene},
	std::time::Duration,
};
pub(crate) use {
	bridge::CanvasPerfSink,
	report::{PerfDashboard, PerfGraphSeries},
};

#[derive(Debug, Default)]
pub(crate) struct PerfMonitor {
	store: PerfStore,
	sink: Sink,
}

impl PerfMonitor {
	pub(crate) fn sink(&self) -> CanvasPerfSink {
		self.sink.clone()
	}

	pub(crate) fn record_editor_apply(&mut self, duration: Duration) {
		self.store.record_editor_apply(duration);
	}

	pub(crate) fn record_editor_command(&mut self, duration: Duration) {
		self.store.record_editor_command(duration);
	}

	pub(crate) fn record_scene_build(&mut self, duration: Duration) {
		self.store.record_scene_build(duration);
	}

	pub(crate) fn record_resize_reflow(&mut self, duration: Duration) {
		self.store.record_resize_reflow(duration);
	}

	pub(crate) fn flush_canvas_metrics(&mut self) {
		self.store.flush_canvas_metrics(&self.sink);
	}

	pub(crate) fn dashboard(&self, scene: &LayoutScene, editor_mode: EditorMode, editor_bytes: usize) -> PerfDashboard {
		build_dashboard(&self.store, scene, editor_mode, editor_bytes)
	}
}
