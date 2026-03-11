//! Low-level text and glyph playground.
//!
//! This example sits between `iced` widget code and the text stack underneath
//! it. It is intentionally not a full custom renderer. The point is to expose
//! the seams:
//!
//! - edit text locally
//! - shape and lay it out with `cosmic-text`
//! - inspect runs and glyphs directly
//! - compare that data against `canvas::Text`
//! - draw vendored glyph outlines from `swash`
//!
//! # Upstream map
//!
//! ## Already adapted locally
//!
//! These upstream pieces are copied or re-expressed in this file.
//!
//! 1. Font to `cosmic-text::Attrs`
//!    - Upstream: `iced_graphics::text`
//!    - Local: `to_attributes`, `to_family`, `to_weight`, `to_stretch`,
//!      `to_style`
//!    - Why: this keeps our shaping inputs close to what `iced` itself uses.
//!
//! 2. Swash outline traversal
//!    - Upstream: `iced_graphics::geometry::text::Text::draw_with`
//!    - Local: outline extraction in `LayoutScene::build`
//!    - Why: this is the core of the local outline rendering mode.
//!
//! ## Still external and vital
//!
//! These are still runtime-defining and are not vendored here.
//!
//! 1. Canvas widget event model
//!    - Upstream: `iced_widget::canvas::program`, `iced_widget::action`
//!    - Affects: `GlyphCanvas` interaction and message publishing
//!    - Why it matters: hover/click behavior still follows Iced's widget
//!      runtime contracts.
//!
//! 2. `canvas::Text` renderer path
//!    - Upstream: `iced_wgpu::geometry`, `iced_wgpu::text`,
//!      `iced_graphics::text::cache`
//!    - Affects: the blue `canvas::Text` overlay
//!    - Why it matters: caching, clipping, atlas upload, and final GPU text
//!      rendering are still upstream-owned.
//!
//! 3. `cosmic-text` layout model
//!    - Upstream: `cosmic_text::buffer`, `cosmic_text::layout`
//!    - Affects: `LayoutRun`, `LayoutGlyph`, `glyph.physical(...)`
//!    - Why it matters: our hit-testing, dumps, and glyph boxes all depend on
//!      these structures and their semantics.
//!
//! 4. `cosmic-text::FontSystem` fallback behavior
//!    - Upstream: `cosmic_text::font::system`
//!    - Affects: actual face resolution and fallback
//!    - Why it matters: `make_font_system()` only augments the database; it
//!      does not replace fallback policy.
//!
//! ## Best snipe targets
//!
//! If we want to reduce upstream dependency in order of leverage:
//!
//! 1. `canvas::Text` renderer path
//! 2. `cosmic-text` layout/run abstraction
//! 3. `cosmic-text` font fallback policy
//! 4. Canvas event/action API
//!
//! ## Not worth adapting yet
//!
//! - Font to `Attrs` conversion: already local
//! - Swash outline traversal: already local
//! - Hover/click hit-testing: already local
use iced::advanced::text::{Alignment, LineHeight, Shaping, Wrapping};
use iced::alignment;
use iced::widget::{
	button, canvas, checkbox, column, container, pick_list, row, scrollable, slider, text, text_editor,
};
use iced::{Color, Element, Font, Length, Pixels, Point, Rectangle, Size, Task, Theme, mouse};

use cosmic_text::{Attrs, Buffer, Command, FontSystem, Metrics, SwashCache};

use std::fmt::{self, Display, Write as _};
use std::ops::Range;
const SIDEBAR_WIDTH: f32 = 380.0;
const CONTROL_LABEL_WIDTH: f32 = 90.0;

pub fn run() -> iced::Result {
	let settings = iced::Settings {
		default_font: Font::with_name("Noto Sans CJK SC"),
		..Default::default()
	};

	iced::application(Playground::new, Playground::update, Playground::view)
		.theme(app_theme)
		.settings(settings)
		.run()
}

#[allow(dead_code)]
fn main() -> iced::Result {
	run()
}

fn app_theme(_playground: &Playground) -> Theme {
	Theme::TokyoNightStorm
}

struct Playground {
	source: text_editor::Content,
	preset: SamplePreset,
	font: FontChoice,
	shaping: ShapingChoice,
	wrapping: WrapChoice,
	render_mode: RenderMode,
	font_size: f32,
	line_height: f32,
	layout_width: f32,
	show_baselines: bool,
	show_hitboxes: bool,
	active_sidebar_tab: SidebarTab,
	hovered_target: Option<CanvasTarget>,
	selected_target: Option<CanvasTarget>,
	scene: LayoutScene,
	font_system: FontSystem,
}

#[derive(Debug, Clone)]
enum Message {
	Edit(text_editor::Action),
	LoadPreset(SamplePreset),
	FontSelected(FontChoice),
	ShapingSelected(ShapingChoice),
	WrappingSelected(WrapChoice),
	RenderModeSelected(RenderMode),
	FontSizeChanged(f32),
	LineHeightChanged(f32),
	LayoutWidthChanged(f32),
	ShowBaselinesChanged(bool),
	ShowHitboxesChanged(bool),
	SelectSidebarTab(SidebarTab),
	CanvasHovered(Option<CanvasTarget>),
	CanvasSelected(Option<CanvasTarget>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SidebarTab {
	Controls,
	Inspect,
	Dump,
}

impl SidebarTab {
	const ALL: [Self; 3] = [Self::Controls, Self::Inspect, Self::Dump];
}

impl Display for SidebarTab {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let label = match self {
			Self::Controls => "Controls",
			Self::Inspect => "Inspect",
			Self::Dump => "Dump",
		};

		f.write_str(label)
	}
}

impl Playground {
	fn new() -> (Self, Task<Message>) {
		let mut font_system = make_font_system();
		let preset = SamplePreset::Mixed;
		let source = text_editor::Content::with_text(preset.text());
		let font = FontChoice::JetBrainsMono;
		let shaping = ShapingChoice::Advanced;
		let wrapping = WrapChoice::Word;
		let render_mode = RenderMode::CanvasAndOutlines;
		let font_size = 24.0;
		let line_height = 32.0;
		let layout_width = 540.0;
		let show_baselines = true;
		let show_hitboxes = true;
		let active_sidebar_tab = SidebarTab::Controls;
		let scene = LayoutScene::build(
			&mut font_system,
			source.text(),
			font,
			shaping,
			wrapping,
			font_size,
			line_height,
			layout_width,
			render_mode,
		);

		(
			Self {
				source,
				preset,
				font,
				shaping,
				wrapping,
				render_mode,
				font_size,
				line_height,
				layout_width,
				show_baselines,
				show_hitboxes,
				active_sidebar_tab,
				hovered_target: None,
				selected_target: None,
				scene,
				font_system,
			},
			Task::none(),
		)
	}

	fn update(&mut self, message: Message) -> Task<Message> {
		match message {
			Message::Edit(action) => {
				self.source.perform(action);
				self.preset = SamplePreset::Custom;
				self.refresh_scene();
			}
			Message::LoadPreset(preset) => {
				self.preset = preset;
				if !matches!(preset, SamplePreset::Custom) {
					self.source = text_editor::Content::with_text(preset.text());
					self.refresh_scene();
				}
			}
			Message::FontSelected(font) => {
				self.font = font;
				self.refresh_scene();
			}
			Message::ShapingSelected(shaping) => {
				self.shaping = shaping;
				self.refresh_scene();
			}
			Message::WrappingSelected(wrapping) => {
				self.wrapping = wrapping;
				self.refresh_scene();
			}
			Message::RenderModeSelected(render_mode) => {
				self.render_mode = render_mode;
				self.refresh_scene();
			}
			Message::FontSizeChanged(font_size) => {
				self.font_size = font_size;
				self.line_height = self.line_height.max(self.font_size);
				self.refresh_scene();
			}
			Message::LineHeightChanged(line_height) => {
				self.line_height = line_height;
				self.refresh_scene();
			}
			Message::LayoutWidthChanged(layout_width) => {
				self.layout_width = layout_width;
				self.refresh_scene();
			}
			Message::ShowBaselinesChanged(show_baselines) => {
				self.show_baselines = show_baselines;
			}
			Message::ShowHitboxesChanged(show_hitboxes) => {
				self.show_hitboxes = show_hitboxes;
			}
			Message::SelectSidebarTab(tab) => {
				self.active_sidebar_tab = tab;
			}
			Message::CanvasHovered(target) => {
				self.hovered_target = target;
			}
			Message::CanvasSelected(target) => {
				self.selected_target = target;
			}
		}

		Task::none()
	}

	fn view(&self) -> Element<'_, Message> {
		container(row![self.view_sidebar(), self.view_canvas_pane()].spacing(16))
			.padding(16)
			.width(Length::Fill)
			.height(Length::Fill)
			.into()
	}

	fn view_sidebar(&self) -> Element<'_, Message> {
		container(
			column![
				text("Glyph Playground").size(28),
				text(
					"Iced + cosmic-text + swash. Edit the source, then inspect the shaped runs, glyph boxes, and vendored outlines."
				)
				.size(15),
				self.view_sidebar_tabs(),
				container(self.view_sidebar_body()).height(Length::Fill),
			]
			.spacing(12)
			.padding(16),
		)
		.width(SIDEBAR_WIDTH)
		.height(Length::Fill)
		.style(surface_style)
		.into()
	}

	fn view_sidebar_tabs(&self) -> Element<'_, Message> {
		row(SidebarTab::ALL
			.into_iter()
			.map(|tab| view_sidebar_tab(tab, tab == self.active_sidebar_tab))
			.collect::<Vec<_>>())
		.spacing(2)
		.into()
	}

	fn view_sidebar_body(&self) -> Element<'_, Message> {
		match self.active_sidebar_tab {
			SidebarTab::Controls => self.view_controls_tab(),
			SidebarTab::Inspect => self.view_inspect_tab(),
			SidebarTab::Dump => self.view_dump_tab(),
		}
	}

	fn view_controls_tab(&self) -> Element<'_, Message> {
		scrollable(
			column![
				control_row(
					"Preset",
					pick_list(SamplePreset::ALL, Some(self.preset), Message::LoadPreset,)
						.width(Length::Fill)
						.into(),
				),
				control_row(
					"Font",
					pick_list(FontChoice::ALL, Some(self.font), Message::FontSelected)
						.width(Length::Fill)
						.into(),
				),
				control_row(
					"Shaping",
					pick_list(ShapingChoice::ALL, Some(self.shaping), Message::ShapingSelected,)
						.width(Length::Fill)
						.into(),
				),
				control_row(
					"Wrap",
					pick_list(WrapChoice::ALL, Some(self.wrapping), Message::WrappingSelected,)
						.width(Length::Fill)
						.into(),
				),
				control_row(
					"Render",
					pick_list(RenderMode::ALL, Some(self.render_mode), Message::RenderModeSelected,)
						.width(Length::Fill)
						.into(),
				),
				control_row(
					format!("Size {:.0}", self.font_size),
					slider(10.0..=48.0, self.font_size, Message::FontSizeChanged)
						.width(Length::Fill)
						.into(),
				),
				control_row(
					format!("Line {:.0}", self.line_height),
					slider(12.0..=72.0, self.line_height, Message::LineHeightChanged)
						.width(Length::Fill)
						.into(),
				),
				control_row(
					format!("Width {:.0}", self.layout_width),
					slider(180.0..=900.0, self.layout_width, Message::LayoutWidthChanged)
						.width(Length::Fill)
						.into(),
				),
				checkbox(self.show_baselines)
					.label("Show baselines and line tops")
					.on_toggle(Message::ShowBaselinesChanged),
				checkbox(self.show_hitboxes)
					.label("Show glyph hitboxes")
					.on_toggle(Message::ShowHitboxesChanged),
				text("Source").size(18),
				self.view_source_editor(),
			]
			.spacing(14),
		)
		.into()
	}

	fn view_source_editor(&self) -> Element<'_, Message> {
		text_editor(&self.source)
			.on_action(Message::Edit)
			.font(self.font.to_iced_font())
			.wrapping(self.wrapping.to_iced())
			.line_height(LineHeight::Absolute(Pixels(self.line_height)))
			.size(Pixels((self.font_size * 0.68).max(14.0)))
			.height(220)
			.into()
	}

	fn view_inspect_tab(&self) -> Element<'_, Message> {
		scrollable(
			column![
				text("Warnings").size(18),
				self.view_warnings_panel(),
				text("Hover and selection").size(18),
				self.view_interaction_panel(),
			]
			.spacing(12),
		)
		.into()
	}

	fn view_warnings_panel(&self) -> Element<'_, Message> {
		let warnings_text = if self.scene.warnings.is_empty() {
			"No warnings".to_string()
		} else {
			self.scene.warnings.join("\n")
		};
		let has_warnings = !self.scene.warnings.is_empty();

		container(text(warnings_text).size(14).width(Length::Fill))
			.padding(12)
			.style(move |theme: &Theme| {
				let palette = theme.extended_palette();
				container::Style {
					background: Some(
						if has_warnings {
							palette.warning.weak.color
						} else {
							palette.background.weak.color
						}
						.into(),
					),
					border: iced::Border {
						color: if has_warnings {
							palette.warning.strong.color
						} else {
							palette.background.strong.color
						},
						width: 1.0,
						radius: 8.0.into(),
					},
					..Default::default()
				}
			})
			.into()
	}

	fn view_interaction_panel(&self) -> Element<'_, Message> {
		container(
			scrollable(
				text(self.interaction_details())
					.font(Font::MONOSPACE)
					.size(14)
					.width(Length::Fill),
			)
			.height(Length::Shrink),
		)
		.padding(12)
		.style(panel_style)
		.into()
	}

	fn view_dump_tab(&self) -> Element<'_, Message> {
		container(
			scrollable(
				text(self.scene.dump.clone())
					.font(Font::MONOSPACE)
					.size(14)
					.width(Length::Fill),
			)
			.height(Length::Fill),
		)
		.padding(12)
		.height(Length::Fill)
		.style(panel_style)
		.into()
	}

	fn view_canvas_pane(&self) -> Element<'_, Message> {
		let canvas_view = canvas(GlyphCanvas {
			scene: self.scene.clone(),
			show_baselines: self.show_baselines,
			show_hitboxes: self.show_hitboxes,
			hovered_target: self.hovered_target,
			selected_target: self.selected_target,
		})
		.width(Length::Fill)
		.height(Length::Fill);

		container(canvas_view)
			.padding(8)
			.width(Length::Fill)
			.height(Length::Fill)
			.style(surface_style)
			.into()
	}

	fn refresh_scene(&mut self) {
		self.scene = LayoutScene::build(
			&mut self.font_system,
			self.source.text(),
			self.font,
			self.shaping,
			self.wrapping,
			self.font_size,
			self.line_height,
			self.layout_width,
			self.render_mode,
		);
		self.hovered_target = None;
		self.selected_target = None;
	}

	fn interaction_details(&self) -> String {
		let mut details = String::new();
		let _ = writeln!(details, "hover");
		let _ = writeln!(
			details,
			"{}",
			self.scene
				.target_details(self.hovered_target)
				.unwrap_or_else(|| "  none".to_string())
		);
		let _ = writeln!(details);
		let _ = writeln!(details, "selection");
		let _ = writeln!(
			details,
			"{}",
			self.scene
				.target_details(self.selected_target)
				.unwrap_or_else(|| "  none".to_string())
		);
		details
	}
}

fn control_row<'a>(label: impl Into<String>, control: Element<'a, Message>) -> Element<'a, Message> {
	row![text(label.into()).width(CONTROL_LABEL_WIDTH), control]
		.spacing(12)
		.align_y(iced::Center)
		.into()
}

fn panel_style(theme: &Theme) -> container::Style {
	let palette = theme.extended_palette();
	container::Style {
		background: Some(palette.background.weak.color.into()),
		border: iced::Border {
			color: palette.background.strong.color,
			width: 1.0,
			radius: 8.0.into(),
		},
		..Default::default()
	}
}

fn surface_style(theme: &Theme) -> container::Style {
	let palette = theme.extended_palette();
	container::Style {
		background: Some(palette.background.base.color.into()),
		border: iced::Border {
			color: palette.background.strong.color,
			width: 1.0,
			radius: 8.0.into(),
		},
		..Default::default()
	}
}

fn view_sidebar_tab(tab: SidebarTab, is_active: bool) -> Element<'static, Message> {
	let label_text = text(tab.to_string()).size(14).style(move |theme: &Theme| {
		let palette = theme.extended_palette();
		text::Style {
			color: Some(if is_active {
				palette.background.base.text
			} else {
				let mut color = palette.background.base.text;
				color.a = 0.82;
				color
			}),
		}
	});

	let indicator: Element<'static, Message> = if is_active {
		container(iced::widget::Space::new().width(Length::Fill).height(2))
			.style(move |theme: &Theme| {
				let palette = theme.extended_palette();
				container::Style {
					background: Some(palette.primary.base.color.into()),
					..Default::default()
				}
			})
			.into()
	} else {
		container(iced::widget::Space::new().width(Length::Fill).height(2)).into()
	};

	let content = column![
		indicator,
		container(
			button(
				container(label_text)
					.width(Length::Fill)
					.height(Length::Fill)
					.center_x(Length::Fill)
					.center_y(Length::Fill),
			)
			.on_press(Message::SelectSidebarTab(tab))
			.width(Length::Fill)
			.height(Length::Fill)
			.style(move |theme: &Theme, status| {
				let palette = theme.extended_palette();
				let mut overlay = palette.background.strong.color;
				overlay.a = if is_active {
					0.0
				} else {
					match status {
						button::Status::Hovered => 0.18,
						button::Status::Pressed => 0.24,
						_ => 0.0,
					}
				};

				button::Style {
					background: Some(overlay.into()),
					..button::text(theme, status)
				}
			})
		)
		.padding([3, 10])
		.height(Length::Fill)
		.width(Length::Fill)
	]
	.spacing(0);

	container(content)
		.height(38)
		.width(Length::Fill)
		.style(move |theme: &Theme| {
			let palette = theme.extended_palette();
			container::Style {
				background: Some(
					if is_active {
						palette.background.base.color
					} else {
						palette.background.weak.color
					}
					.into(),
				),
				border: iced::Border {
					width: 1.0,
					color: palette.background.strong.color,
					..iced::Border::default()
				},
				..Default::default()
			}
		})
		.into()
}

#[derive(Debug, Clone)]
struct GlyphCanvas {
	scene: LayoutScene,
	show_baselines: bool,
	show_hitboxes: bool,
	hovered_target: Option<CanvasTarget>,
	selected_target: Option<CanvasTarget>,
}

#[derive(Debug, Default)]
struct CanvasState {
	hovered_target: Option<CanvasTarget>,
}

// Canvas interaction still relies on Iced's widget/runtime contracts.
impl canvas::Program<Message> for GlyphCanvas {
	type State = CanvasState;

	fn update(
		&self, state: &mut Self::State, event: &canvas::Event, bounds: Rectangle, cursor: mouse::Cursor,
	) -> Option<canvas::Action<Message>> {
		let cursor_target = cursor.position_in(bounds).and_then(|position| self.hit_test(position));

		match event {
			canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
				if state.hovered_target != cursor_target {
					state.hovered_target = cursor_target;
					return Some(canvas::Action::publish(Message::CanvasHovered(cursor_target)));
				}
			}
			canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
				if cursor.is_over(bounds) {
					state.hovered_target = cursor_target;
					return Some(canvas::Action::publish(Message::CanvasSelected(cursor_target)).and_capture());
				}
			}
			canvas::Event::Mouse(mouse::Event::ButtonPressed(_)) => {
				if cursor.is_over(bounds) {
					return Some(canvas::Action::capture());
				}
			}
			_ => {
				if !cursor.is_over(bounds) && state.hovered_target.is_some() {
					state.hovered_target = None;
					return Some(canvas::Action::publish(Message::CanvasHovered(None)));
				}
			}
		}

		None
	}

	fn draw(
		&self, _state: &Self::State, renderer: &iced::Renderer, _theme: &Theme, bounds: Rectangle,
		_cursor: iced::mouse::Cursor,
	) -> Vec<canvas::Geometry> {
		let mut frame = canvas::Frame::new(renderer, bounds.size());
		let origin = scene_origin();
		let text_area_top_left = origin;
		let text_area_size = Size::new(
			self.scene.max_width.max(1.0),
			self.scene.measured_height.max(bounds.height - origin.y - 24.0),
		);

		frame.fill_rectangle(Point::ORIGIN, bounds.size(), Color::from_rgb8(20, 24, 32));
		frame.fill_rectangle(text_area_top_left, text_area_size, Color::from_rgb8(28, 34, 46));
		frame.stroke_rectangle(
			text_area_top_left,
			Size::new(self.scene.max_width.max(1.0), self.scene.measured_height.max(1.0)),
			canvas::Stroke::default()
				.with_width(1.0)
				.with_color(Color::from_rgba(0.8, 0.8, 0.9, 0.65)),
		);

		let guide = canvas::Path::line(Point::new(origin.x, 0.0), Point::new(origin.x, bounds.height));
		frame.stroke(
			&guide,
			canvas::Stroke::default()
				.with_width(1.0)
				.with_color(Color::from_rgba(0.6, 0.7, 1.0, 0.18)),
		);

		if self.scene.draw_canvas_text {
			let max_width = if self.scene.canvas_wraps {
				self.scene.max_width
			} else {
				f32::INFINITY
			};

			frame.fill_text(canvas::Text {
				content: self.scene.text.clone(),
				position: origin,
				max_width,
				color: Color::from_rgba(0.4, 0.8, 1.0, 0.9),
				size: Pixels(self.scene.font_size),
				line_height: LineHeight::Absolute(Pixels(self.scene.line_height)),
				font: self.scene.font,
				align_x: Alignment::Left,
				align_y: alignment::Vertical::Top,
				shaping: self.scene.shaping.to_iced(),
			});
		}

		for (run_index, run) in self.scene.runs.iter().enumerate() {
			if self.selected_target == Some(CanvasTarget::Run(run_index)) {
				frame.fill_rectangle(
					Point::new(origin.x, origin.y + run.line_top),
					Size::new(
						self.scene.max_width.max(run.line_width).max(1.0),
						run.line_height.max(1.0),
					),
					Color::from_rgba(1.0, 0.85, 0.2, 0.14),
				);
			} else if self.hovered_target == Some(CanvasTarget::Run(run_index)) {
				frame.fill_rectangle(
					Point::new(origin.x, origin.y + run.line_top),
					Size::new(
						self.scene.max_width.max(run.line_width).max(1.0),
						run.line_height.max(1.0),
					),
					Color::from_rgba(0.4, 0.8, 1.0, 0.1),
				);
			}

			if self.show_baselines {
				let top_line = canvas::Path::line(
					Point::new(origin.x, origin.y + run.line_top),
					Point::new(origin.x + self.scene.max_width, origin.y + run.line_top),
				);
				frame.stroke(
					&top_line,
					canvas::Stroke::default()
						.with_width(1.0)
						.with_color(Color::from_rgba(1.0, 0.6, 0.2, 0.45)),
				);

				let baseline = canvas::Path::line(
					Point::new(origin.x, origin.y + run.baseline),
					Point::new(origin.x + self.scene.max_width, origin.y + run.baseline),
				);
				frame.stroke(
					&baseline,
					canvas::Stroke::default()
						.with_width(1.0)
						.with_color(Color::from_rgba(0.4, 1.0, 0.6, 0.45)),
				);
			}

			for (glyph_index, glyph) in run.glyphs.iter().enumerate() {
				let target = CanvasTarget::Glyph { run_index, glyph_index };

				if self.selected_target == Some(target) {
					frame.fill_rectangle(
						Point::new(origin.x + glyph.x, origin.y + glyph.y),
						Size::new(glyph.width.max(1.0), glyph.height.max(1.0)),
						Color::from_rgba(1.0, 0.85, 0.2, 0.25),
					);
				} else if self.hovered_target == Some(target) {
					frame.fill_rectangle(
						Point::new(origin.x + glyph.x, origin.y + glyph.y),
						Size::new(glyph.width.max(1.0), glyph.height.max(1.0)),
						Color::from_rgba(0.4, 0.8, 1.0, 0.18),
					);
				}

				if self.show_hitboxes {
					frame.stroke_rectangle(
						Point::new(origin.x + glyph.x, origin.y + glyph.y),
						Size::new(glyph.width.max(0.5), glyph.height.max(0.5)),
						canvas::Stroke::default()
							.with_width(1.0)
							.with_color(if self.selected_target == Some(target) {
								Color::from_rgba(1.0, 0.9, 0.2, 0.95)
							} else if self.hovered_target == Some(target) {
								Color::from_rgba(0.5, 0.85, 1.0, 0.95)
							} else {
								Color::from_rgba(1.0, 0.3, 0.3, 0.6)
							}),
					);
				}

				if self.scene.draw_outlines {
					if let Some(outline) = &glyph.outline {
						let path = canvas::Path::new(|builder| {
							for command in &outline.commands {
								match command {
									PathCommand::MoveTo(point) => {
										builder.move_to(Point::new(origin.x + point.x, origin.y + point.y))
									}
									PathCommand::LineTo(point) => {
										builder.line_to(Point::new(origin.x + point.x, origin.y + point.y))
									}
									PathCommand::QuadTo(control, to) => builder.quadratic_curve_to(
										Point::new(origin.x + control.x, origin.y + control.y),
										Point::new(origin.x + to.x, origin.y + to.y),
									),
									PathCommand::CurveTo(a, b, to) => builder.bezier_curve_to(
										Point::new(origin.x + a.x, origin.y + a.y),
										Point::new(origin.x + b.x, origin.y + b.y),
										Point::new(origin.x + to.x, origin.y + to.y),
									),
									PathCommand::Close => builder.close(),
								}
							}
						});

						frame.fill(&path, Color::from_rgb8(245, 245, 240));
					} else {
						frame.fill_rectangle(
							Point::new(origin.x + glyph.x, origin.y + glyph.y),
							Size::new(glyph.width.max(1.0), glyph.height.max(1.0)),
							Color::from_rgba(0.95, 0.9, 0.3, 0.18),
						);
					}
				}
			}
		}

		let footer = format!(
			"runs={} glyphs={} fonts={} width={:.1} height={:.1}",
			self.scene.runs.len(),
			self.scene.glyph_count,
			self.scene.fonts_seen.len(),
			self.scene.measured_width,
			self.scene.measured_height,
		);
		frame.fill_text(canvas::Text {
			content: footer,
			position: Point::new(24.0, bounds.height - 24.0),
			color: Color::from_rgb8(180, 190, 210),
			size: Pixels(14.0),
			font: Font::MONOSPACE,
			..canvas::Text::default()
		});

		vec![frame.into_geometry()]
	}

	fn mouse_interaction(&self, _state: &Self::State, bounds: Rectangle, cursor: mouse::Cursor) -> mouse::Interaction {
		cursor
			.position_in(bounds)
			.and_then(|position| self.hit_test(position))
			.map(|_| mouse::Interaction::Pointer)
			.unwrap_or_default()
	}
}

impl GlyphCanvas {
	fn hit_test(&self, cursor_position: Point) -> Option<CanvasTarget> {
		let local = Point::new(
			cursor_position.x - scene_origin().x,
			cursor_position.y - scene_origin().y,
		);
		self.scene.hit_test(local)
	}
}

#[derive(Debug, Clone)]
struct LayoutScene {
	text: String,
	font: Font,
	shaping: ShapingChoice,
	font_size: f32,
	line_height: f32,
	max_width: f32,
	measured_width: f32,
	measured_height: f32,
	glyph_count: usize,
	runs: Vec<RunInfo>,
	fonts_seen: Vec<String>,
	warnings: Vec<String>,
	dump: String,
	draw_canvas_text: bool,
	draw_outlines: bool,
	canvas_wraps: bool,
}

impl LayoutScene {
	#[allow(clippy::too_many_arguments)]
	// This scene builder is where our local code meets upstream `cosmic-text`.
	// The layout model itself is not vendored: `Buffer`, `LayoutRun`,
	// `LayoutGlyph`, and fallback still come from `cosmic-text`.
	fn build(
		font_system: &mut FontSystem, text: String, font_choice: FontChoice, shaping: ShapingChoice,
		wrapping: WrapChoice, font_size: f32, line_height: f32, max_width: f32, render_mode: RenderMode,
	) -> Self {
		let mut buffer = Buffer::new(font_system, Metrics::new(font_size, line_height));
		buffer.set_size(font_system, Some(max_width), None);
		buffer.set_wrap(font_system, wrapping.to_cosmic());
		buffer.set_text(
			font_system,
			&text,
			&to_attributes(font_choice.to_iced_font()),
			shaping.to_cosmic(&text),
			None,
		);

		let mut swash_cache = SwashCache::new();
		let mut runs = Vec::new();
		let mut warnings = Vec::new();
		let mut fonts_seen = Vec::<String>::new();
		let mut measured_width: f32 = 0.0;
		let mut measured_height: f32 = 0.0;
		let mut glyph_count = 0usize;

		for run in buffer.layout_runs() {
			measured_width = measured_width.max(run.line_w);
			measured_height = measured_height.max(run.line_top + run.line_height);

			let mut glyphs = Vec::new();
			for glyph in run.glyphs {
				glyph_count += 1;

				let font_name = font_system
					.db()
					.face(glyph.font_id)
					.map(|face| face.post_script_name.clone())
					.unwrap_or_else(|| format!("font#{:?}", glyph.font_id));

				if !fonts_seen.iter().any(|existing| existing == &font_name) {
					fonts_seen.push(font_name.clone());
				}

				let cluster_text = run
					.text
					.get(glyph.start..glyph.end)
					.map(debug_snippet)
					.unwrap_or_else(|| "<invalid utf8 slice>".to_string());

				let physical_glyph = glyph.physical((0.0, 0.0), 1.0);
				let outline = swash_cache
					.get_outline_commands(font_system, physical_glyph.cache_key)
					.map(|commands| OutlinePath {
						commands: commands
							.iter()
							.map(|command| match command {
								Command::MoveTo(point) => PathCommand::MoveTo(PathPoint {
									x: point.x + glyph.x + glyph.x_offset,
									y: -point.y + run.line_y + glyph.y_offset,
								}),
								Command::LineTo(point) => PathCommand::LineTo(PathPoint {
									x: point.x + glyph.x + glyph.x_offset,
									y: -point.y + run.line_y + glyph.y_offset,
								}),
								Command::QuadTo(control, to) => PathCommand::QuadTo(
									PathPoint {
										x: control.x + glyph.x + glyph.x_offset,
										y: -control.y + run.line_y + glyph.y_offset,
									},
									PathPoint {
										x: to.x + glyph.x + glyph.x_offset,
										y: -to.y + run.line_y + glyph.y_offset,
									},
								),
								Command::CurveTo(a, b, to) => PathCommand::CurveTo(
									PathPoint {
										x: a.x + glyph.x + glyph.x_offset,
										y: -a.y + run.line_y + glyph.y_offset,
									},
									PathPoint {
										x: b.x + glyph.x + glyph.x_offset,
										y: -b.y + run.line_y + glyph.y_offset,
									},
									PathPoint {
										x: to.x + glyph.x + glyph.x_offset,
										y: -to.y + run.line_y + glyph.y_offset,
									},
								),
								Command::Close => PathCommand::Close,
							})
							.collect(),
					});

				glyphs.push(GlyphInfo {
					cluster: cluster_text,
					cluster_range: glyph.start..glyph.end,
					x: glyph.x,
					y: run.line_top + glyph.y,
					width: glyph.w,
					height: glyph.line_height_opt.unwrap_or(run.line_height),
					glyph_id: glyph.glyph_id,
					font_name,
					font_size: glyph.font_size,
					x_offset: glyph.x_offset,
					y_offset: glyph.y_offset,
					outline,
				});
			}

			runs.push(RunInfo {
				line_index: run.line_i,
				rtl: run.rtl,
				baseline: run.line_y,
				line_top: run.line_top,
				line_height: run.line_height,
				line_width: run.line_w,
				glyphs,
			});
		}

		if matches!(render_mode, RenderMode::CanvasOnly | RenderMode::CanvasAndOutlines)
			&& matches!(wrapping, WrapChoice::Glyph | WrapChoice::WordOrGlyph)
		{
			warnings.push(
                "The blue overlay uses `canvas::Text`, which only exposes Iced's default wrapping behavior. Glyph-level wrapping is only reflected in the outline and dump.".to_string(),
            );
		}

		if runs.is_empty() {
			warnings.push("No layout runs were produced. Check the font choice and text content.".to_string());
		}

		let dump = build_dump(
			&text,
			font_choice,
			shaping,
			wrapping,
			render_mode,
			font_size,
			line_height,
			max_width,
			measured_width,
			measured_height,
			glyph_count,
			&fonts_seen,
			&runs,
		);

		Self {
			text,
			font: font_choice.to_iced_font(),
			shaping,
			font_size,
			line_height,
			max_width,
			measured_width,
			measured_height,
			glyph_count,
			runs,
			fonts_seen,
			warnings,
			dump,
			draw_canvas_text: render_mode.draw_canvas_text(),
			draw_outlines: render_mode.draw_outlines(),
			canvas_wraps: !matches!(wrapping, WrapChoice::None),
		}
	}

	fn hit_test(&self, local: Point) -> Option<CanvasTarget> {
		for (run_index, run) in self.runs.iter().enumerate() {
			for (glyph_index, glyph) in run.glyphs.iter().enumerate() {
				if contains_point(local, glyph.x, glyph.y, glyph.width.max(1.0), glyph.height.max(1.0)) {
					return Some(CanvasTarget::Glyph { run_index, glyph_index });
				}
			}

			if contains_point(
				local,
				0.0,
				run.line_top,
				self.max_width.max(run.line_width).max(1.0),
				run.line_height.max(1.0),
			) {
				return Some(CanvasTarget::Run(run_index));
			}
		}

		None
	}

	fn target_details(&self, target: Option<CanvasTarget>) -> Option<String> {
		match target? {
			CanvasTarget::Run(run_index) => {
				let run = self.runs.get(run_index)?;
				Some(format!(
					"  kind: run\n  run index: {run_index}\n  source line: {}\n  rtl: {}\n  top: {:.1}\n  baseline: {:.1}\n  height: {:.1}\n  width: {:.1}\n  glyphs: {}",
					run.line_index,
					run.rtl,
					run.line_top,
					run.baseline,
					run.line_height,
					run.line_width,
					run.glyphs.len(),
				))
			}
			CanvasTarget::Glyph { run_index, glyph_index } => {
				let run = self.runs.get(run_index)?;
				let glyph = run.glyphs.get(glyph_index)?;
				Some(format!(
					"  kind: glyph\n  run index: {run_index}\n  glyph index: {glyph_index}\n  source line: {}\n  cluster: {}\n  bytes: {:?}\n  font: {}\n  glyph id: {}\n  x/y: {:.1}, {:.1}\n  w/h: {:.1}, {:.1}\n  size: {:.1}\n  x/y offset: {:.3}, {:.3}\n  outline: {}",
					run.line_index,
					glyph.cluster,
					glyph.cluster_range,
					glyph.font_name,
					glyph.glyph_id,
					glyph.x,
					glyph.y,
					glyph.width,
					glyph.height,
					glyph.font_size,
					glyph.x_offset,
					glyph.y_offset,
					glyph.outline.is_some(),
				))
			}
		}
	}
}

#[derive(Debug, Clone)]
struct RunInfo {
	line_index: usize,
	rtl: bool,
	baseline: f32,
	line_top: f32,
	line_height: f32,
	line_width: f32,
	glyphs: Vec<GlyphInfo>,
}

#[derive(Debug, Clone)]
struct GlyphInfo {
	cluster: String,
	cluster_range: Range<usize>,
	x: f32,
	y: f32,
	width: f32,
	height: f32,
	glyph_id: u16,
	font_name: String,
	font_size: f32,
	x_offset: f32,
	y_offset: f32,
	outline: Option<OutlinePath>,
}

#[derive(Debug, Clone)]
struct OutlinePath {
	commands: Vec<PathCommand>,
}

#[derive(Debug, Clone)]
enum PathCommand {
	MoveTo(PathPoint),
	LineTo(PathPoint),
	QuadTo(PathPoint, PathPoint),
	CurveTo(PathPoint, PathPoint, PathPoint),
	Close,
}

#[derive(Debug, Clone, Copy)]
struct PathPoint {
	x: f32,
	y: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CanvasTarget {
	Run(usize),
	Glyph { run_index: usize, glyph_index: usize },
}

fn scene_origin() -> Point {
	Point::new(24.0, 28.0)
}

fn contains_point(point: Point, x: f32, y: f32, width: f32, height: f32) -> bool {
	point.x >= x && point.x <= x + width && point.y >= y && point.y <= y + height
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SamplePreset {
	Mixed,
	Rust,
	Ligatures,
	Arabic,
	Cjk,
	Emoji,
	Custom,
}

impl SamplePreset {
	const ALL: [SamplePreset; 7] = [
		SamplePreset::Mixed,
		SamplePreset::Rust,
		SamplePreset::Ligatures,
		SamplePreset::Arabic,
		SamplePreset::Cjk,
		SamplePreset::Emoji,
		SamplePreset::Custom,
	];

	fn text(self) -> &'static str {
		match self {
			SamplePreset::Mixed => "office affine ffi ffl\n漢字カタカナ and Latin\nالسلام عليكم\nemoji 🙂🚀👩‍💻",
			SamplePreset::Rust => "fn main() {\n    println!(\"ffi -> office -> 汉字\");\n}\n",
			SamplePreset::Ligatures => "office affine final fluff ffi ffl fj",
			SamplePreset::Arabic => "السلام عليكم\nمرحبا بالعالم",
			SamplePreset::Cjk => "漢字かなカナ\n混在テキスト with ASCII",
			SamplePreset::Emoji => "🙂🚀👩‍💻 text + emoji fallback",
			SamplePreset::Custom => "",
		}
	}
}

impl Display for SamplePreset {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			SamplePreset::Mixed => write!(f, "Mixed"),
			SamplePreset::Rust => write!(f, "Rust"),
			SamplePreset::Ligatures => write!(f, "Ligatures"),
			SamplePreset::Arabic => write!(f, "Arabic"),
			SamplePreset::Cjk => write!(f, "CJK"),
			SamplePreset::Emoji => write!(f, "Emoji"),
			SamplePreset::Custom => write!(f, "Custom"),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FontChoice {
	JetBrainsMono,
	Monospace,
	NotoSansCjk,
	SansSerif,
}

impl FontChoice {
	const ALL: [FontChoice; 4] = [
		FontChoice::JetBrainsMono,
		FontChoice::Monospace,
		FontChoice::NotoSansCjk,
		FontChoice::SansSerif,
	];

	fn to_iced_font(self) -> Font {
		match self {
			FontChoice::JetBrainsMono => Font::with_name("JetBrains Mono"),
			FontChoice::Monospace => Font::MONOSPACE,
			FontChoice::NotoSansCjk => Font::with_name("Noto Sans CJK SC"),
			FontChoice::SansSerif => Font::DEFAULT,
		}
	}
}

impl Display for FontChoice {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			FontChoice::JetBrainsMono => write!(f, "JetBrains Mono"),
			FontChoice::Monospace => write!(f, "Monospace family"),
			FontChoice::NotoSansCjk => write!(f, "Noto Sans CJK SC"),
			FontChoice::SansSerif => write!(f, "Sans Serif family"),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShapingChoice {
	Auto,
	Basic,
	Advanced,
}

impl ShapingChoice {
	const ALL: [ShapingChoice; 3] = [ShapingChoice::Auto, ShapingChoice::Basic, ShapingChoice::Advanced];

	fn to_iced(self) -> Shaping {
		match self {
			ShapingChoice::Auto => Shaping::Auto,
			ShapingChoice::Basic => Shaping::Basic,
			ShapingChoice::Advanced => Shaping::Advanced,
		}
	}

	fn to_cosmic(self, text: &str) -> cosmic_text::Shaping {
		match self {
			ShapingChoice::Auto => {
				if text.is_ascii() {
					cosmic_text::Shaping::Basic
				} else {
					cosmic_text::Shaping::Advanced
				}
			}
			ShapingChoice::Basic => cosmic_text::Shaping::Basic,
			ShapingChoice::Advanced => cosmic_text::Shaping::Advanced,
		}
	}
}

impl Display for ShapingChoice {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			ShapingChoice::Auto => write!(f, "Auto"),
			ShapingChoice::Basic => write!(f, "Basic"),
			ShapingChoice::Advanced => write!(f, "Advanced"),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WrapChoice {
	None,
	Word,
	Glyph,
	WordOrGlyph,
}

impl WrapChoice {
	const ALL: [WrapChoice; 4] = [
		WrapChoice::None,
		WrapChoice::Word,
		WrapChoice::Glyph,
		WrapChoice::WordOrGlyph,
	];

	fn to_iced(self) -> Wrapping {
		match self {
			WrapChoice::None => Wrapping::None,
			WrapChoice::Word => Wrapping::Word,
			WrapChoice::Glyph => Wrapping::Glyph,
			WrapChoice::WordOrGlyph => Wrapping::WordOrGlyph,
		}
	}

	fn to_cosmic(self) -> cosmic_text::Wrap {
		match self {
			WrapChoice::None => cosmic_text::Wrap::None,
			WrapChoice::Word => cosmic_text::Wrap::Word,
			WrapChoice::Glyph => cosmic_text::Wrap::Glyph,
			WrapChoice::WordOrGlyph => cosmic_text::Wrap::WordOrGlyph,
		}
	}
}

impl Display for WrapChoice {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			WrapChoice::None => write!(f, "None"),
			WrapChoice::Word => write!(f, "Word"),
			WrapChoice::Glyph => write!(f, "Glyph"),
			WrapChoice::WordOrGlyph => write!(f, "Word or glyph"),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RenderMode {
	CanvasOnly,
	OutlinesOnly,
	CanvasAndOutlines,
}

impl RenderMode {
	const ALL: [RenderMode; 3] = [
		RenderMode::CanvasOnly,
		RenderMode::OutlinesOnly,
		RenderMode::CanvasAndOutlines,
	];

	fn draw_canvas_text(self) -> bool {
		matches!(self, RenderMode::CanvasOnly | RenderMode::CanvasAndOutlines)
	}

	fn draw_outlines(self) -> bool {
		matches!(self, RenderMode::OutlinesOnly | RenderMode::CanvasAndOutlines)
	}
}

impl Display for RenderMode {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			RenderMode::CanvasOnly => write!(f, "canvas::Text"),
			RenderMode::OutlinesOnly => write!(f, "Outlines"),
			RenderMode::CanvasAndOutlines => write!(f, "Both"),
		}
	}
}

// We extend `cosmic-text::FontSystem` with local demo fonts, but fallback and
// DB behavior remain upstream-owned. See `cosmic-text/src/font/system.rs`.
fn make_font_system() -> FontSystem {
	let mut font_system = FontSystem::new();
	let db = font_system.db_mut();
	db.set_monospace_family("JetBrains Mono");
	db.set_sans_serif_family("Noto Sans CJK SC");
	font_system
}

// Adapted from `iced_graphics/src/text.rs` so the example can shape text
// without reaching back into `iced_graphics` internals.
fn to_attributes(font: Font) -> Attrs<'static> {
	Attrs::new()
		.family(to_family(font.family))
		.weight(to_weight(font.weight))
		.stretch(to_stretch(font.stretch))
		.style(to_style(font.style))
}

fn to_family(family: iced::font::Family) -> cosmic_text::Family<'static> {
	match family {
		iced::font::Family::Name(name) => cosmic_text::Family::Name(name),
		iced::font::Family::SansSerif => cosmic_text::Family::SansSerif,
		iced::font::Family::Serif => cosmic_text::Family::Serif,
		iced::font::Family::Cursive => cosmic_text::Family::Cursive,
		iced::font::Family::Fantasy => cosmic_text::Family::Fantasy,
		iced::font::Family::Monospace => cosmic_text::Family::Monospace,
	}
}

fn to_weight(weight: iced::font::Weight) -> cosmic_text::Weight {
	match weight {
		iced::font::Weight::Thin => cosmic_text::Weight::THIN,
		iced::font::Weight::ExtraLight => cosmic_text::Weight::EXTRA_LIGHT,
		iced::font::Weight::Light => cosmic_text::Weight::LIGHT,
		iced::font::Weight::Normal => cosmic_text::Weight::NORMAL,
		iced::font::Weight::Medium => cosmic_text::Weight::MEDIUM,
		iced::font::Weight::Semibold => cosmic_text::Weight::SEMIBOLD,
		iced::font::Weight::Bold => cosmic_text::Weight::BOLD,
		iced::font::Weight::ExtraBold => cosmic_text::Weight::EXTRA_BOLD,
		iced::font::Weight::Black => cosmic_text::Weight::BLACK,
	}
}

fn to_stretch(stretch: iced::font::Stretch) -> cosmic_text::Stretch {
	match stretch {
		iced::font::Stretch::UltraCondensed => cosmic_text::Stretch::UltraCondensed,
		iced::font::Stretch::ExtraCondensed => cosmic_text::Stretch::ExtraCondensed,
		iced::font::Stretch::Condensed => cosmic_text::Stretch::Condensed,
		iced::font::Stretch::SemiCondensed => cosmic_text::Stretch::SemiCondensed,
		iced::font::Stretch::Normal => cosmic_text::Stretch::Normal,
		iced::font::Stretch::SemiExpanded => cosmic_text::Stretch::SemiExpanded,
		iced::font::Stretch::Expanded => cosmic_text::Stretch::Expanded,
		iced::font::Stretch::ExtraExpanded => cosmic_text::Stretch::ExtraExpanded,
		iced::font::Stretch::UltraExpanded => cosmic_text::Stretch::UltraExpanded,
	}
}

fn to_style(style: iced::font::Style) -> cosmic_text::Style {
	match style {
		iced::font::Style::Normal => cosmic_text::Style::Normal,
		iced::font::Style::Italic => cosmic_text::Style::Italic,
		iced::font::Style::Oblique => cosmic_text::Style::Oblique,
	}
}

#[allow(clippy::too_many_arguments)]
fn build_dump(
	text_value: &str, font: FontChoice, shaping: ShapingChoice, wrapping: WrapChoice, render_mode: RenderMode,
	font_size: f32, line_height: f32, max_width: f32, measured_width: f32, measured_height: f32, glyph_count: usize,
	fonts_seen: &[String], runs: &[RunInfo],
) -> String {
	let mut dump = String::new();

	let _ = writeln!(dump, "config");
	let _ = writeln!(dump, "  font: {font}");
	let _ = writeln!(dump, "  shaping: {shaping}");
	let _ = writeln!(dump, "  wrapping: {wrapping}");
	let _ = writeln!(dump, "  render mode: {render_mode}");
	let _ = writeln!(dump, "  text length: {} bytes", text_value.len());
	let _ = writeln!(dump, "  font size: {:.1}", font_size);
	let _ = writeln!(dump, "  line height: {:.1}", line_height);
	let _ = writeln!(dump, "  max width: {:.1}", max_width);
	let _ = writeln!(dump, "  measured width: {:.1}", measured_width);
	let _ = writeln!(dump, "  measured height: {:.1}", measured_height);
	let _ = writeln!(dump, "  runs: {}", runs.len());
	let _ = writeln!(dump, "  glyphs: {glyph_count}");
	let _ = writeln!(dump, "  fonts used: {}", fonts_seen.join(", "));
	let _ = writeln!(dump);

	let glyph_limit = 220usize;
	let mut emitted = 0usize;

	for (run_index, run) in runs.iter().enumerate() {
		let _ = writeln!(
			dump,
			"run {run_index}: line={} rtl={} top={:.1} baseline={:.1} height={:.1} width={:.1} glyphs={}",
			run.line_index,
			run.rtl,
			run.line_top,
			run.baseline,
			run.line_height,
			run.line_width,
			run.glyphs.len(),
		);

		for glyph in &run.glyphs {
			if emitted >= glyph_limit {
				let remaining = glyph_count.saturating_sub(emitted);
				let _ = writeln!(dump, "  ... truncated {remaining} more glyphs");
				return dump;
			}

			emitted += 1;
			let _ = writeln!(
				dump,
				"  glyph {}: cluster={} bytes={:?} font={} glyph_id={} x={:.1} y={:.1} w={:.1} h={:.1} size={:.1} x_off={:.3} y_off={:.3} outline={}",
				emitted - 1,
				glyph.cluster,
				glyph.cluster_range,
				glyph.font_name,
				glyph.glyph_id,
				glyph.x,
				glyph.y,
				glyph.width,
				glyph.height,
				glyph.font_size,
				glyph.x_offset,
				glyph.y_offset,
				glyph.outline.is_some(),
			);
		}

		let _ = writeln!(dump);
	}

	dump
}

fn debug_snippet(text: &str) -> String {
	let escaped: String = text.chars().flat_map(char::escape_default).collect();

	if escaped.is_empty() {
		"<empty>".to_string()
	} else {
		format!("\"{escaped}\"")
	}
}
