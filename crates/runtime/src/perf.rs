#[derive(Debug, Clone, Default, PartialEq)]
pub struct PerfProjection {
	pub scene_builds: usize,
	pub scene_build_millis: f64,
}

impl PerfProjection {
	pub fn record_scene_build(&mut self, millis: f64) {
		self.scene_builds += 1;
		self.scene_build_millis += millis;
	}
}
