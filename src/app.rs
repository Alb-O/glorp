use iced::widget::responsive;
use iced::{Element, Length, Task};

use std::fmt::Write as _;

use crate::editor::EditorBuffer;
use crate::scene::{LayoutScene, make_font_system};
use crate::types::{FontChoice, Message, RenderMode, SamplePreset, ShapingChoice, SidebarTab, WrapChoice};
use crate::ui::{
	CanvasPaneProps, ControlsTabProps, InspectTabProps, SidebarProps, view_canvas_pane, view_controls_tab,
	view_dump_tab, view_inspect_tab, view_shell, view_sidebar,
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
		responsive(|size| {
			let stacked = size.width < 1120.0;
			let sidebar = view_sidebar(SidebarProps {
				active_tab: self.active_sidebar_tab,
				editor_mode: self.editor.mode(),
				editor_bytes: self.editor.text().len(),
				body: self.view_sidebar_body(),
				stacked,
			});
			let canvas = view_canvas_pane(CanvasPaneProps {
				scene: self.scene.clone(),
				show_baselines: self.show_baselines,
				show_hitboxes: self.show_hitboxes,
				hovered_target: self.hovered_target,
				selected_target: self.selected_target,
				editor: self.editor.view_state(),
				scene_revision: self.scene_revision,
				stacked,
			});

			view_shell(size, sidebar, canvas)
		})
		.width(Length::Fill)
		.height(Length::Fill)
		.into()
	}

	fn view_sidebar_body(&self) -> Element<'_, Message> {
		match self.active_sidebar_tab {
			SidebarTab::Controls => view_controls_tab(ControlsTabProps {
				preset: self.preset,
				font: self.font,
				shaping: self.shaping,
				wrapping: self.wrapping,
				render_mode: self.render_mode,
				font_size: self.font_size,
				line_height: self.line_height,
				layout_width: self.layout_width,
				show_baselines: self.show_baselines,
				show_hitboxes: self.show_hitboxes,
			}),
			SidebarTab::Inspect => view_inspect_tab(InspectTabProps {
				warnings: &self.scene.warnings,
				interaction_details: self.interaction_details(),
			}),
			SidebarTab::Dump => view_dump_tab(&self.scene.dump),
		}
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
