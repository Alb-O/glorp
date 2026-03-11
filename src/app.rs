use iced::widget::{canvas, checkbox, column, container, pick_list, responsive, row, scrollable, slider, text};
use iced::{Element, Font, Length, Size, Task, Theme};

use std::fmt::Write as _;

use crate::canvas_view::GlyphCanvas;
use crate::editor::EditorBuffer;
use crate::scene::{LayoutScene, make_font_system};
use crate::types::{FontChoice, Message, RenderMode, SamplePreset, ShapingChoice, SidebarTab, WrapChoice};
use crate::ui::{
	CONTROL_RADIUS, SIDEBAR_WIDTH, control_row, panel_style, rounded_checkbox_style, rounded_pick_list_menu_style,
	rounded_pick_list_style, rounded_slider_style, surface_style, view_sidebar_tab,
};

pub(crate) struct Playground {
	editor: EditorBuffer,
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
	hovered_target: Option<crate::types::CanvasTarget>,
	selected_target: Option<crate::types::CanvasTarget>,
	scene: LayoutScene,
	font_system: cosmic_text::FontSystem,
	scene_revision: u64,
}

const STACK_LAYOUT_BREAKPOINT: f32 = 1120.0;

impl Playground {
	pub(crate) fn new() -> (Self, Task<Message>) {
		let mut font_system = make_font_system();
		let preset = SamplePreset::Mixed;
		let editor = EditorBuffer::new(preset.text());
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
			editor.text().to_string(),
			font,
			shaping,
			wrapping,
			font_size,
			line_height,
			layout_width,
			render_mode,
		);
		let mut editor = editor;
		editor.sync_with_scene(&scene);

		(
			Self {
				editor,
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
				scene_revision: 1,
			},
			Task::none(),
		)
	}

	pub(crate) fn update(&mut self, message: Message) -> Task<Message> {
		match message {
			Message::LoadPreset(preset) => {
				self.preset = preset;
				if !matches!(preset, SamplePreset::Custom) {
					self.editor.reset(preset.text());
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
				self.scene_revision += 1;
			}
			Message::ShowHitboxesChanged(show_hitboxes) => {
				self.show_hitboxes = show_hitboxes;
				self.scene_revision += 1;
			}
			Message::SelectSidebarTab(tab) => {
				self.active_sidebar_tab = tab;
			}
			Message::CanvasHovered(target) => {
				self.hovered_target = target;
			}
			Message::CanvasClicked { target, position } => {
				self.selected_target = target;
				self.editor
					.apply(crate::editor::EditorCommand::SelectClusterAt(position), &self.scene);
				self.sync_selected_target();
			}
			Message::EditorCommand(command) => {
				let changed = self.editor.apply(command, &self.scene);
				self.preset = SamplePreset::Custom;
				if changed {
					self.refresh_scene();
				} else {
					self.sync_selected_target();
				}
			}
		}

		Task::none()
	}

	pub(crate) fn view(&self) -> Element<'_, Message> {
		responsive(|size| self.view_root(size))
			.width(Length::Fill)
			.height(Length::Fill)
			.into()
	}

	fn view_root(&self, size: Size) -> Element<'_, Message> {
		let stacked = size.width < STACK_LAYOUT_BREAKPOINT;
		let content: Element<'_, Message> = if stacked {
			column![self.view_canvas_pane(true), self.view_sidebar(true),]
				.spacing(12)
				.into()
		} else {
			row![self.view_sidebar(false), self.view_canvas_pane(false),]
				.spacing(16)
				.into()
		};

		container(content)
			.padding(16)
			.width(Length::Fill)
			.height(Length::Fill)
			.into()
	}

	fn view_sidebar(&self, stacked: bool) -> Element<'_, Message> {
		container(
			column![
				text("Glyph Playground").size(28),
				text(
					"Iced + cosmic-text + swash. Edit the source, then inspect the shaped runs, glyph boxes, and vendored outlines."
				)
				.size(15),
				self.view_sidebar_tabs(),
				self.view_editor_status(),
				container(self.view_sidebar_body()).height(Length::Fill),
			]
			.spacing(12)
			.padding(16),
		)
		.width(if stacked {
			Length::Fill
		} else {
			Length::Fixed(SIDEBAR_WIDTH)
		})
		.height(if stacked { Length::FillPortion(2) } else { Length::Fill })
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
					pick_list(SamplePreset::ALL, Some(self.preset), Message::LoadPreset)
						.style(rounded_pick_list_style)
						.menu_style(rounded_pick_list_menu_style)
						.width(Length::Fill)
						.into(),
				),
				control_row(
					"Font",
					pick_list(FontChoice::ALL, Some(self.font), Message::FontSelected)
						.style(rounded_pick_list_style)
						.menu_style(rounded_pick_list_menu_style)
						.width(Length::Fill)
						.into(),
				),
				control_row(
					"Shaping",
					pick_list(ShapingChoice::ALL, Some(self.shaping), Message::ShapingSelected)
						.style(rounded_pick_list_style)
						.menu_style(rounded_pick_list_menu_style)
						.width(Length::Fill)
						.into(),
				),
				control_row(
					"Wrap",
					pick_list(WrapChoice::ALL, Some(self.wrapping), Message::WrappingSelected)
						.style(rounded_pick_list_style)
						.menu_style(rounded_pick_list_menu_style)
						.width(Length::Fill)
						.into(),
				),
				control_row(
					"Render",
					pick_list(RenderMode::ALL, Some(self.render_mode), Message::RenderModeSelected)
						.style(rounded_pick_list_style)
						.menu_style(rounded_pick_list_menu_style)
						.width(Length::Fill)
						.into(),
				),
				control_row(
					format!("Size {:.0}", self.font_size),
					slider(10.0..=48.0, self.font_size, Message::FontSizeChanged)
						.style(rounded_slider_style)
						.width(Length::Fill)
						.into(),
				),
				control_row(
					format!("Line {:.0}", self.line_height),
					slider(12.0..=72.0, self.line_height, Message::LineHeightChanged)
						.style(rounded_slider_style)
						.width(Length::Fill)
						.into(),
				),
				control_row(
					format!("Width {:.0}", self.layout_width),
					slider(180.0..=900.0, self.layout_width, Message::LayoutWidthChanged)
						.style(rounded_slider_style)
						.width(Length::Fill)
						.into(),
				),
				checkbox(self.show_baselines)
					.label("Show baselines and line tops")
					.style(rounded_checkbox_style)
					.on_toggle(Message::ShowBaselinesChanged),
				checkbox(self.show_hitboxes)
					.label("Show glyph hitboxes")
					.style(rounded_checkbox_style)
					.on_toggle(Message::ShowHitboxesChanged),
				text("Canvas editor").size(18),
				self.view_editor_help(),
			]
			.spacing(14),
		)
		.into()
	}

	fn view_editor_status(&self) -> Element<'_, Message> {
		container(
			text(format!(
				"Editor: {} mode, {} bytes",
				self.editor.mode(),
				self.editor.text().len()
			))
			.font(Font::MONOSPACE)
			.size(14),
		)
		.padding([0, 2])
		.into()
	}

	fn view_editor_help(&self) -> Element<'_, Message> {
		container(
			text(
				"Click the canvas to focus.\nNormal: h/j/k/l or arrows move, i inserts before, a inserts after, x deletes.\nInsert: type, Enter/Tab insert text, Backspace/Delete edit, Esc returns to normal mode."
			)
			.size(14)
			.width(Length::Fill),
		)
		.padding(12)
		.style(panel_style)
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
						radius: CONTROL_RADIUS.into(),
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

	fn view_canvas_pane(&self, stacked: bool) -> Element<'_, Message> {
		let canvas_view = canvas(GlyphCanvas {
			scene: self.scene.clone(),
			show_baselines: self.show_baselines,
			show_hitboxes: self.show_hitboxes,
			hovered_target: self.hovered_target,
			selected_target: self.selected_target,
			editor: self.editor.view_state(),
			scene_revision: self.scene_revision,
		})
		.width(Length::Fill)
		.height(Length::Fill);

		container(canvas_view)
			.padding(8)
			.width(Length::Fill)
			.height(if stacked { Length::FillPortion(3) } else { Length::Fill })
			.style(surface_style)
			.into()
	}

	fn refresh_scene(&mut self) {
		self.scene = LayoutScene::build(
			&mut self.font_system,
			self.editor.text().to_string(),
			self.font,
			self.shaping,
			self.wrapping,
			self.font_size,
			self.line_height,
			self.layout_width,
			self.render_mode,
		);
		self.editor.sync_with_scene(&self.scene);
		self.hovered_target = None;
		self.sync_selected_target();
		self.scene_revision += 1;
	}

	fn sync_selected_target(&mut self) {
		self.selected_target = self
			.editor
			.view_state()
			.selection
			.as_ref()
			.and_then(|selection| self.scene.cluster_index_for_range(selection))
			.and_then(|index| self.scene.cluster(index))
			.map(|cluster| crate::types::CanvasTarget::Glyph {
				run_index: cluster.run_index,
				glyph_index: cluster.glyph_start,
			});
	}

	fn interaction_details(&self) -> String {
		let mut details = String::new();
		let _ = writeln!(details, "editor");
		let _ = writeln!(details, "{}", self.editor.selection_details(&self.scene));
		let _ = writeln!(details);
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
