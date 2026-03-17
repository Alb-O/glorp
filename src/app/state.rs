use {
	crate::{
		canvas_view::scene_viewport_size,
		editor::EditorViewportMetrics,
		overlay::LayoutRect,
		scene::{DocumentLayout, SceneConfig, scene_config},
		types::{CanvasTarget, FontChoice, SamplePreset, ShapingChoice, SidebarTab, WrapChoice},
		ui::default_sidebar_ratio,
	},
	iced::{Size, Vector, widget::pane_grid},
	std::time::Duration,
};

pub(super) const RESIZE_REFLOW_INTERVAL: Duration = Duration::from_millis(16);
const DEFAULT_CANVAS_HEIGHT: f32 = 320.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ShellPane {
	Sidebar,
	Canvas,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EditorDispatchSource {
	Keyboard,
	PointerPress,
	PointerDrag,
	PointerRelease,
}

impl EditorDispatchSource {
	pub(super) fn reveals_viewport(self) -> bool {
		matches!(self, Self::Keyboard | Self::PointerPress)
	}
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ResizeCoalescer {
	applied_width: f32,
	pending_width: Option<f32>,
}

impl ResizeCoalescer {
	pub(super) fn new(width: f32) -> Self {
		Self {
			applied_width: width,
			pending_width: None,
		}
	}

	pub(super) fn observe(&mut self, width: f32) {
		if (self.applied_width - width).abs() < 0.5 && self.pending_width.is_none() {
			return;
		}

		self.pending_width = Some(width);
	}

	pub(super) fn flush(&mut self) -> Option<f32> {
		let width = self.pending_width?;
		self.pending_width = None;

		if (self.applied_width - width).abs() < 0.5 {
			return None;
		}

		self.applied_width = width;
		Some(width)
	}

	pub(super) fn has_pending(&self) -> bool {
		self.pending_width.is_some()
	}

	pub(super) fn mark_applied(&mut self, width: f32) {
		self.applied_width = width;

		if self.pending_width.is_some_and(|pending| (pending - width).abs() < 0.5) {
			self.pending_width = None;
		}
	}
}

#[derive(Debug)]
pub(super) struct ControlsState {
	pub(super) preset: SamplePreset,
	pub(super) font: FontChoice,
	pub(super) shaping: ShapingChoice,
	pub(super) wrapping: WrapChoice,
	pub(super) font_size: f32,
	pub(super) line_height: f32,
	pub(super) show_baselines: bool,
	pub(super) show_hitboxes: bool,
}

impl ControlsState {
	pub(super) fn new() -> Self {
		Self {
			preset: SamplePreset::Tall,
			font: FontChoice::JetBrainsMono,
			shaping: ShapingChoice::Advanced,
			wrapping: WrapChoice::Word,
			font_size: 24.0,
			line_height: 32.0,
			show_baselines: false,
			show_hitboxes: false,
		}
	}

	pub(super) const fn initial_layout_width() -> f32 {
		540.0
	}

	pub(super) fn scene_config(&self, layout_width: f32) -> SceneConfig {
		scene_config(
			self.font,
			self.shaping,
			self.wrapping,
			self.font_size,
			self.line_height,
			layout_width,
		)
	}
}

#[derive(Debug)]
pub(super) struct ViewportState {
	pub(super) layout_width: f32,
	pub(super) canvas_viewport: Size,
	pub(super) canvas_focused: bool,
	pub(super) canvas_scroll: Vector,
	pub(super) scene_revision: u64,
	pub(super) resize_coalescer: ResizeCoalescer,
}

impl ViewportState {
	pub(super) fn new(layout_width: f32) -> Self {
		Self {
			layout_width,
			canvas_viewport: Size::new(layout_width, DEFAULT_CANVAS_HEIGHT),
			canvas_focused: false,
			canvas_scroll: Vector::ZERO,
			scene_revision: 0,
			resize_coalescer: ResizeCoalescer::new(layout_width),
		}
	}

	pub(super) fn observe_resize(&mut self, size: Size) -> bool {
		let viewport = scene_viewport_size(size);
		let layout_width = viewport.width;
		let width_changed = (self.layout_width - layout_width).abs() >= 0.5;

		self.canvas_viewport = viewport;
		self.layout_width = layout_width;
		self.resize_coalescer.observe(layout_width);

		width_changed
	}

	pub(super) fn flush_resize(&mut self) -> Option<f32> {
		self.resize_coalescer.flush()
	}

	pub(super) fn mark_scene_applied(&mut self) {
		self.resize_coalescer.mark_applied(self.layout_width);
	}

	pub(super) fn clamp_scroll(&mut self, layout: &DocumentLayout) {
		self.canvas_scroll = self.clamped_scroll(self.canvas_scroll, layout);
	}

	pub(super) fn clamp_scroll_to_metrics(&mut self, metrics: EditorViewportMetrics) {
		self.canvas_scroll = self.clamped_scroll_to_metrics(self.canvas_scroll, metrics);
	}

	pub(super) fn reveal_target_with_metrics(&mut self, target: Option<LayoutRect>, metrics: EditorViewportMetrics) {
		let Some(target) = target else {
			self.clamp_scroll_to_metrics(metrics);
			return;
		};

		let viewport = self.canvas_viewport;
		let mut scroll = self.clamped_scroll_to_metrics(self.canvas_scroll, metrics);
		let margin_x = 24.0;
		let margin_y = 24.0;
		let left = target.x;
		let right = target.x + target.width.max(1.0);
		let top = target.y;
		let bottom = target.y + target.height.max(1.0);

		if left < scroll.x + margin_x {
			scroll.x = (left - margin_x).max(0.0);
		} else if right > scroll.x + viewport.width - margin_x {
			scroll.x = (right - viewport.width + margin_x).max(0.0);
		}

		if top < scroll.y + margin_y {
			scroll.y = (top - margin_y).max(0.0);
		} else if bottom > scroll.y + viewport.height - margin_y {
			scroll.y = (bottom - viewport.height + margin_y).max(0.0);
		}

		self.canvas_scroll = self.clamped_scroll_to_metrics(scroll, metrics);
	}

	pub(super) fn finish_scene_refresh(&mut self, layout: &DocumentLayout, reset_scroll: bool) {
		if reset_scroll {
			self.canvas_scroll = Vector::ZERO;
		}

		self.clamp_scroll(layout);
	}

	pub(super) fn finish_editor_refresh(&mut self, metrics: EditorViewportMetrics, reset_scroll: bool) {
		if reset_scroll {
			self.canvas_scroll = Vector::ZERO;
		}

		self.clamp_scroll_to_metrics(metrics);
	}

	fn clamped_scroll(&self, scroll: Vector, layout: &DocumentLayout) -> Vector {
		self.clamped_scroll_to_metrics(
			scroll,
			EditorViewportMetrics {
				wrapping: layout.wrapping,
				measured_width: layout.measured_width,
				measured_height: layout.measured_height,
			},
		)
	}

	fn clamped_scroll_to_metrics(&self, scroll: Vector, metrics: EditorViewportMetrics) -> Vector {
		let max_x = if matches!(metrics.wrapping, WrapChoice::None) {
			(metrics.measured_width.max(self.layout_width) - self.canvas_viewport.width).max(0.0)
		} else {
			(self.layout_width - self.canvas_viewport.width).max(0.0)
		};
		let max_y = (metrics.measured_height - self.canvas_viewport.height).max(0.0);

		Vector::new(scroll.x.clamp(0.0, max_x), scroll.y.clamp(0.0, max_y))
	}
}

#[derive(Debug)]
pub(super) struct SidebarState {
	pub(super) active_tab: SidebarTab,
	pub(super) hovered_target: Option<CanvasTarget>,
	pub(super) selected_target: Option<CanvasTarget>,
}

impl SidebarState {
	pub(super) fn new() -> Self {
		Self {
			active_tab: SidebarTab::Controls,
			hovered_target: None,
			selected_target: None,
		}
	}

	pub(super) fn set_active_tab(&mut self, tab: SidebarTab) {
		self.active_tab = tab;

		if tab != SidebarTab::Inspect {
			self.clear_inspect_targets();
		}
	}

	pub(super) fn set_hovered_target(&mut self, target: Option<CanvasTarget>) {
		self.hovered_target = self.inspect_target(target);
	}

	pub(super) fn set_selected_target(&mut self, target: Option<CanvasTarget>) {
		self.selected_target = self.inspect_target(target);
	}

	pub(super) fn sync_after_scene_refresh(&mut self) {
		self.hovered_target = None;

		if self.active_tab != SidebarTab::Inspect {
			self.selected_target = None;
		}
	}

	fn clear_inspect_targets(&mut self) {
		self.hovered_target = None;
		self.selected_target = None;
	}

	fn inspect_target(&self, target: Option<CanvasTarget>) -> Option<CanvasTarget> {
		target.filter(|_| self.active_tab == SidebarTab::Inspect)
	}
}

#[derive(Debug)]
pub(super) struct ShellState {
	pub(super) chrome: pane_grid::State<ShellPane>,
}

impl ShellState {
	pub(super) fn new() -> Self {
		Self {
			chrome: pane_grid::State::with_configuration(pane_grid::Configuration::Split {
				axis: pane_grid::Axis::Vertical,
				ratio: default_sidebar_ratio(),
				a: Box::new(pane_grid::Configuration::Pane(ShellPane::Sidebar)),
				b: Box::new(pane_grid::Configuration::Pane(ShellPane::Canvas)),
			}),
		}
	}
}
