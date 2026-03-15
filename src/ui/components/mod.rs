//! Props-based composed views.
//!
//! This is the default UI boundary in `liney`: the parent owns state, and each
//! module renders from explicit props.

mod controls;
mod inspect;
mod perf;
mod shell;
mod sidebar;

pub(crate) use {
	controls::{ControlsTabProps, view_controls_tab},
	inspect::{InspectTabProps, view_inspect_tab},
	perf::{PerfTabProps, view_perf_tab},
	shell::{CanvasPaneProps, default_sidebar_ratio, is_stacked_shell, view_canvas_pane, view_stacked_shell},
	sidebar::{SidebarProps, view_sidebar},
};
