//! UI composition rules for `glorp`.
//!
//! Reusable UI lives in normal Rust modules with explicit inputs. The app keeps
//! state ownership in one place instead of turning each section into an
//! encapsulated mini-application.
//!
//! Preferred order:
//!
//! 1. Use view helpers for stateless composition.
//! 2. Use props-based child modules when a section grows.
//! 3. Keep state in the parent and map child messages or tasks back up.
//! 4. Use generic overlay modules for transient layered UI.
//! 5. Use `lazy(...)` only for expensive derived subtrees.
//! 6. Use custom widgets or `canvas::Program` only when composition is no
//!    longer enough for layout, drawing, or event handling.
//!
//! `iced` `Component`s are not the default here. They hide state ownership, and
//! the installed `iced_widget` source deprecates them for that reason.
//!
//! In this repo:
//!
//! - `app.rs` owns state and update logic.
//! - `ui::components` holds composed views and props structs.
//! - `ui::tokens` holds theme-aware style helpers and tiny primitives.

pub(crate) mod components;
pub(crate) mod tokens;

pub(crate) use {
	components::{
		CanvasDecorations, CanvasPaneProps, ControlsTabProps, InspectTabProps, SidebarProps, default_sidebar_ratio,
		is_stacked_shell, view_canvas_pane, view_controls_tab, view_inspect_tab, view_perf_tab, view_sidebar,
		view_stacked_shell,
	},
	tokens::{
		CONTROL_RADIUS, PICK_LIST_PADDING, control_row, panel_scrollable, panel_style, rounded_checkbox_style,
		rounded_pick_list_menu_style, rounded_pick_list_style, rounded_slider_style, surface_style, view_sidebar_tab,
	},
};
