//! Props-based composed views.
//!
//! This is the default UI boundary in `liney`: the parent owns state, and each
//! module renders from explicit props.

mod controls;
mod dump;
mod inspect;
mod shell;
mod sidebar;

pub(crate) use controls::{ControlsTabProps, view_controls_tab};
pub(crate) use dump::view_dump_tab;
pub(crate) use inspect::{InspectTabProps, view_inspect_tab};
pub(crate) use shell::{
	CanvasPaneProps, default_sidebar_ratio, is_stacked_shell, view_canvas_pane, view_stacked_shell,
};
pub(crate) use sidebar::{SidebarProps, view_sidebar};
