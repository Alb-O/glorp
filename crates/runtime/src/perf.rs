#[derive(Debug, Clone, Default, PartialEq)]
pub struct PerfMetricProjection {
	pub total_samples: u64,
	pub total_millis: f64,
	pub last_millis: f64,
}

impl PerfMetricProjection {
	pub fn record(&mut self, millis: f64) {
		self.total_samples += 1;
		self.total_millis += millis;
		self.last_millis = millis;
	}

	pub fn average_millis(&self) -> f64 {
		if self.total_samples == 0 {
			0.0
		} else {
			self.total_millis / self.total_samples as f64
		}
	}
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct PerfProjection {
	pub scene_build: PerfMetricProjection,
}

impl PerfProjection {
	pub fn record_scene_build(&mut self, millis: f64) {
		self.scene_build.record(millis);
	}
}
