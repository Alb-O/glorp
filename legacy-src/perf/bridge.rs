use {
	crate::telemetry::duration_ms,
	std::{
		collections::VecDeque,
		mem,
		sync::{Arc, Mutex},
		time::{Duration, Instant},
	},
	tracing::{debug, trace, warn},
};

const PENDING_LIMIT: usize = 512;

#[derive(Debug, Default)]
pub(super) struct PendingSamples {
	pub(super) updates: VecDeque<Duration>,
	pub(super) underlay: VecDeque<Duration>,
	pub(super) overlay: VecDeque<Duration>,
	pub(super) draws: VecDeque<CanvasDrawSample>,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct CanvasDrawSample {
	pub(super) total: Duration,
	pub(super) static_build: Option<Duration>,
	pub(super) drawn_at: Instant,
	pub(super) cache_miss: bool,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct CanvasPerfSink {
	pending: Arc<Mutex<PendingSamples>>,
}

impl CanvasPerfSink {
	pub(crate) fn record_canvas_update(&self, duration: Duration) {
		self.record_pending_duration(
			duration,
			|pending| &mut pending.updates,
			"canvas update over frame budget",
			"canvas update over warning threshold",
			"canvas update",
		);
	}

	pub(crate) fn record_canvas_underlay(&self, duration: Duration) {
		self.record_pending_duration(
			duration,
			|pending| &mut pending.underlay,
			"underlay draw over frame budget",
			"underlay draw over warning threshold",
			"underlay draw",
		);
	}

	pub(crate) fn record_canvas_overlay(&self, duration: Duration) {
		self.record_pending_duration(
			duration,
			|pending| &mut pending.overlay,
			"overlay draw over frame budget",
			"overlay draw over warning threshold",
			"overlay draw",
		);
	}

	pub(crate) fn record_canvas_draw(&self, total: Duration, static_build: Option<Duration>, cache_miss: bool) {
		let Ok(mut pending) = self.pending.lock() else {
			return;
		};

		push_bounded(
			&mut pending.draws,
			CanvasDrawSample {
				total,
				static_build,
				drawn_at: Instant::now(),
				cache_miss,
			},
			PENDING_LIMIT,
		);

		log_draw_duration(total, static_build, cache_miss);
	}

	pub(super) fn drain(&self) -> PendingSamples {
		let Ok(mut pending) = self.pending.lock() else {
			return PendingSamples::default();
		};

		PendingSamples {
			updates: mem::take(&mut pending.updates),
			underlay: mem::take(&mut pending.underlay),
			overlay: mem::take(&mut pending.overlay),
			draws: mem::take(&mut pending.draws),
		}
	}
}

impl CanvasPerfSink {
	fn record_pending_duration(
		&self, duration: Duration, slot: impl FnOnce(&mut PendingSamples) -> &mut VecDeque<Duration>,
		over_budget: &'static str, over_warning: &'static str, normal: &'static str,
	) {
		let Ok(mut pending) = self.pending.lock() else {
			return;
		};

		// Keep queueing and threshold logging coupled so the three simple canvas
		// paths cannot drift in behavior as they evolve.
		push_bounded(slot(&mut pending), duration, PENDING_LIMIT);
		log_duration(duration, over_budget, over_warning, normal);
	}
}

fn log_duration(duration: Duration, over_budget: &'static str, over_warning: &'static str, normal: &'static str) {
	let elapsed_ms = duration_ms(duration);
	if elapsed_ms >= 16.7 {
		warn!(duration_ms = elapsed_ms, "{over_budget}");
	} else if elapsed_ms >= 8.0 {
		debug!(duration_ms = elapsed_ms, "{over_warning}");
	} else {
		trace!(duration_ms = elapsed_ms, "{normal}");
	}
}

fn log_draw_duration(total: Duration, static_build: Option<Duration>, cache_miss: bool) {
	let total_ms = duration_ms(total);
	let static_build_ms = static_build.map(duration_ms);

	if total_ms >= 16.7 {
		warn!(total_ms, static_build_ms, cache_miss, "canvas draw over frame budget");
	} else if total_ms >= 8.0 {
		debug!(
			total_ms,
			static_build_ms, cache_miss, "canvas draw over warning threshold"
		);
	} else {
		trace!(total_ms, static_build_ms, cache_miss, "canvas draw");
	}
}

fn push_bounded<T>(items: &mut VecDeque<T>, value: T, limit: usize) {
	if items.len() == limit {
		items.pop_front();
	}

	items.push_back(value);
}

#[cfg(test)]
mod tests {
	use {super::CanvasPerfSink, std::time::Duration};

	#[test]
	fn sink_drain_clears_pending_samples() {
		let sink = CanvasPerfSink::default();
		sink.record_canvas_update(Duration::from_millis(2));
		sink.record_canvas_underlay(Duration::from_millis(1));
		sink.record_canvas_overlay(Duration::from_millis(1));
		sink.record_canvas_draw(Duration::from_millis(3), Some(Duration::from_millis(1)), true);

		let pending = sink.drain();
		assert_eq!(pending.updates.len(), 1);
		assert_eq!(pending.underlay.len(), 1);
		assert_eq!(pending.overlay.len(), 1);
		assert_eq!(pending.draws.len(), 1);

		let drained_again = sink.drain();
		assert!(drained_again.updates.is_empty());
		assert!(drained_again.underlay.is_empty());
		assert!(drained_again.overlay.is_empty());
		assert!(drained_again.draws.is_empty());
	}
}
