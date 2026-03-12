use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const PENDING_LIMIT: usize = 512;

#[derive(Debug, Default)]
pub(super) struct PendingSamples {
	pub(super) canvas_update: VecDeque<Duration>,
	pub(super) canvas_draw: VecDeque<CanvasDrawSample>,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct CanvasDrawSample {
	pub(super) total: Duration,
	pub(super) static_build: Option<Duration>,
	pub(super) overlay: Duration,
	pub(super) drawn_at: Instant,
	pub(super) cache_miss: bool,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct CanvasPerfSink {
	pending: Arc<Mutex<PendingSamples>>,
}

impl CanvasPerfSink {
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

	pub(super) fn drain(&self) -> PendingSamples {
		let Ok(mut pending) = self.pending.lock() else {
			return PendingSamples::default();
		};

		PendingSamples {
			canvas_update: pending.canvas_update.drain(..).collect(),
			canvas_draw: pending.canvas_draw.drain(..).collect(),
		}
	}
}

fn push_bounded<T>(items: &mut VecDeque<T>, value: T, limit: usize) {
	if items.len() == limit {
		let _ = items.pop_front();
	}

	items.push_back(value);
}

#[cfg(test)]
mod tests {
	use super::CanvasPerfSink;
	use std::time::Duration;

	#[test]
	fn sink_drain_clears_pending_samples() {
		let sink = CanvasPerfSink::default();
		sink.record_canvas_update(Duration::from_millis(2));
		sink.record_canvas_draw(
			Duration::from_millis(3),
			Some(Duration::from_millis(1)),
			Duration::from_millis(1),
			true,
		);

		let pending = sink.drain();
		assert_eq!(pending.canvas_update.len(), 1);
		assert_eq!(pending.canvas_draw.len(), 1);

		let drained_again = sink.drain();
		assert!(drained_again.canvas_update.is_empty());
		assert!(drained_again.canvas_draw.is_empty());
	}
}
