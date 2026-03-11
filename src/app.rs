use iced::widget::{pane_grid, responsive};
use iced::{Element, Length, Task};

use std::fmt::Write as _;

use crate::editor::EditorBuffer;
use crate::scene::{LayoutScene, make_font_system};
use crate::types::{FontChoice, Message, RenderMode, SamplePreset, ShapingChoice, SidebarTab, WrapChoice};
use crate::ui::{
	CanvasPaneProps, ControlsTabProps, InspectTabProps, SidebarProps, default_sidebar_ratio, is_stacked_shell,
	view_canvas_pane, view_controls_tab, view_dump_tab, view_inspect_tab, view_sidebar, view_stacked_shell,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShellPane {
	Sidebar,
	Canvas,
}

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
	chrome: pane_grid::State<ShellPane>,
	scene_revision: u64,
}

impl Playground {
	pub(crate) fn new() -> (Self, Task<Message>) {
		let mut font_system = make_font_system();
		let preset = SamplePreset::Tall;
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
		let chrome = pane_grid::State::with_configuration(pane_grid::Configuration::Split {
			axis: pane_grid::Axis::Vertical,
			ratio: default_sidebar_ratio(),
			a: Box::new(pane_grid::Configuration::Pane(ShellPane::Sidebar)),
			b: Box::new(pane_grid::Configuration::Pane(ShellPane::Canvas)),
		});
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
				chrome,
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
			Message::PaneResized(event) => {
				self.chrome.resize(event.split, event.ratio);
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
			if is_stacked_shell(size) {
				let sidebar = self.view_sidebar(true);
				let canvas = self.view_canvas(true);
				return view_stacked_shell(sidebar, canvas);
			}

			let grid = pane_grid(&self.chrome, |_, pane, _| {
				pane_grid::Content::new(match pane {
					ShellPane::Sidebar => self.view_sidebar(false),
					ShellPane::Canvas => self.view_canvas(false),
				})
			})
			.width(Length::Fill)
			.height(Length::Fill)
			.spacing(12)
			.min_size(220)
			.on_resize(12, Message::PaneResized);

			iced::widget::container(grid)
				.padding(16)
				.width(Length::Fill)
				.height(Length::Fill)
				.into()
		})
		.width(Length::Fill)
		.height(Length::Fill)
		.into()
	}

	fn view_sidebar(&self, stacked: bool) -> Element<'_, Message> {
		view_sidebar(SidebarProps {
			active_tab: self.active_sidebar_tab,
			editor_mode: self.editor.mode(),
			editor_bytes: self.editor.text().len(),
			body: self.view_sidebar_body(),
			stacked,
		})
	}

	fn view_canvas(&self, stacked: bool) -> Element<'static, Message> {
		view_canvas_pane(CanvasPaneProps {
			scene: self.scene.clone(),
			show_baselines: self.show_baselines,
			show_hitboxes: self.show_hitboxes,
			hovered_target: self.hovered_target,
			selected_target: self.selected_target,
			editor: self.editor.view_state(),
			scene_revision: self.scene_revision,
			stacked,
		})
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
