use {
	crate::{
		perf::PerfDashboard,
		types::{FontChoice, RenderMode, SamplePreset, ShapingChoice, WrapChoice},
	},
	std::sync::Arc,
};

#[derive(Debug, Clone, Copy)]
pub(super) struct ControlsSidebarData {
	pub(super) preset: SamplePreset,
	pub(super) font: FontChoice,
	pub(super) shaping: ShapingChoice,
	pub(super) wrapping: WrapChoice,
	pub(super) render_mode: RenderMode,
	pub(super) font_size: f32,
	pub(super) line_height: f32,
	pub(super) show_baselines: bool,
	pub(super) show_hitboxes: bool,
}

#[derive(Debug, Clone)]
pub(super) struct InspectSidebarData {
	pub(super) warnings: Arc<[String]>,
	pub(super) interaction_details: String,
}

#[derive(Debug, Clone)]
pub(super) struct PerfSidebarData {
	pub(super) dashboard: PerfDashboard,
}

#[derive(Debug, Clone)]
pub(super) enum SidebarBodyData {
	Controls(ControlsSidebarData),
	Inspect(Arc<InspectSidebarData>),
	Perf(Arc<PerfSidebarData>),
}
