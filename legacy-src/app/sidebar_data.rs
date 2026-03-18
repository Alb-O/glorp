use {crate::ui::ControlsTabProps, std::sync::Arc};

#[derive(Debug, Clone)]
pub(super) struct InspectSidebarData {
	pub(super) warnings: Arc<[String]>,
	pub(super) interaction_details: Arc<str>,
}

#[derive(Debug, Clone)]
pub(super) enum SidebarBodyData {
	Controls(ControlsTabProps),
	Inspect(Arc<InspectSidebarData>),
	Perf(Arc<crate::perf::PerfDashboard>),
}
