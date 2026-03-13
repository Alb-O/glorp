#![allow(dead_code, missing_docs, unused_imports, unused_mut)]

#[path = "../src/app/mod.rs"]
mod app;
#[path = "../src/canvas_view/mod.rs"]
mod canvas_view;
#[path = "../src/editor/mod.rs"]
mod editor;
#[path = "../src/overlay.rs"]
mod overlay;
#[path = "../src/perf/mod.rs"]
mod perf;
#[path = "../src/scene/mod.rs"]
mod scene;
#[path = "../src/text_view.rs"]
mod text_view;
#[path = "../src/types.rs"]
mod types;
#[path = "../src/ui/mod.rs"]
mod ui;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use iced::advanced::renderer::{Headless, Style};
use iced::mouse;
use iced::{Color, Font, Pixels, Size, Theme};
use iced_runtime::{UserInterface, user_interface};
use pollster::block_on;

use std::env;
use std::hint::black_box;

use app::Playground;
use types::{CanvasEvent, ControlsMessage, Message, SamplePreset, SidebarMessage, SidebarTab, ViewportMessage};

criterion_group!(benches, benchmark_headless_playground);
criterion_main!(benches);

const VIEWPORT_SCALE_FACTOR: f32 = 1.0;
const VIEWPORT_PHYSICAL_WIDTH: u32 = 1600;
const VIEWPORT_PHYSICAL_HEIGHT: u32 = 1000;

fn benchmark_headless_playground(c: &mut Criterion) {
	let mut group = c.benchmark_group("playground/headless");

	for scenario in Scenario::ALL {
		let mut harness = Harness::new(scenario);
		let renderer = harness.renderer_name.clone();

		group.bench_with_input(BenchmarkId::new(renderer, scenario.label()), &scenario, |b, _| {
			b.iter(|| {
				black_box(harness.render_frame());
			});
		});
	}

	group.finish();
}

#[derive(Debug, Clone, Copy)]
enum Scenario {
	Default,
	Tall,
	TallInspect,
	TallPerf,
}

impl Scenario {
	const ALL: [Self; 4] = [Self::Default, Self::Tall, Self::TallInspect, Self::TallPerf];

	fn label(self) -> &'static str {
		match self {
			Self::Default => "default",
			Self::Tall => "tall",
			Self::TallInspect => "tall-inspect",
			Self::TallPerf => "tall-perf",
		}
	}
}

struct Harness {
	playground: Playground,
	renderer: iced::Renderer,
	renderer_name: String,
	cache: user_interface::Cache,
	viewport_physical: Size<u32>,
	viewport_logical: Size,
	theme: Theme,
}

impl Harness {
	fn new(scenario: Scenario) -> Self {
		let backend = env::var("LINEY_HEADLESS_BACKEND").ok();
		let mut renderer = block_on(<iced::Renderer as Headless>::new(
			Font::DEFAULT,
			Pixels::from(16),
			backend.as_deref(),
		))
		.unwrap_or_else(|| panic!("failed to create headless iced renderer for backend {backend:?}"));
		let renderer_name = renderer.name();
		let viewport_physical = Size::new(VIEWPORT_PHYSICAL_WIDTH, VIEWPORT_PHYSICAL_HEIGHT);
		let viewport_logical = Size::new(
			VIEWPORT_PHYSICAL_WIDTH as f32 / VIEWPORT_SCALE_FACTOR,
			VIEWPORT_PHYSICAL_HEIGHT as f32 / VIEWPORT_SCALE_FACTOR,
		);
		let (mut playground, _) = Playground::new();

		let mut harness = Self {
			playground,
			renderer,
			renderer_name,
			cache: user_interface::Cache::default(),
			viewport_physical,
			viewport_logical,
			theme: Theme::TokyoNightStorm,
		};
		harness.configure(scenario);
		harness
	}

	fn configure(&mut self, scenario: Scenario) {
		self.apply(Message::Viewport(ViewportMessage::CanvasResized(self.viewport_logical)));

		match scenario {
			Scenario::Default => {}
			Scenario::Tall => {
				self.apply(Message::Controls(ControlsMessage::LoadPreset(SamplePreset::Tall)));
			}
			Scenario::TallInspect => {
				self.apply(Message::Controls(ControlsMessage::LoadPreset(SamplePreset::Tall)));
				self.apply(Message::Controls(ControlsMessage::ShowHitboxesChanged(true)));
				self.apply(Message::Controls(ControlsMessage::ShowBaselinesChanged(true)));
				self.apply(Message::Sidebar(SidebarMessage::SelectTab(SidebarTab::Inspect)));
				self.apply(Message::Canvas(CanvasEvent::Hovered(Some(
					types::CanvasTarget::Glyph {
						run_index: 0,
						glyph_index: 0,
					},
				))));
			}
			Scenario::TallPerf => {
				self.apply(Message::Controls(ControlsMessage::LoadPreset(SamplePreset::Tall)));
				self.apply(Message::Sidebar(SidebarMessage::SelectTab(SidebarTab::Perf)));
			}
		}
	}

	fn render_frame(&mut self) -> usize {
		let mut user_interface = UserInterface::build(
			self.playground.view(),
			self.viewport_logical,
			std::mem::take(&mut self.cache),
			&mut self.renderer,
		);

		user_interface.draw(
			&mut self.renderer,
			&self.theme,
			&Style {
				text_color: Color::WHITE,
			},
			mouse::Cursor::Unavailable,
		);

		self.cache = user_interface.into_cache();

		self.renderer
			.screenshot(self.viewport_physical, VIEWPORT_SCALE_FACTOR, Color::BLACK)
			.len()
	}

	fn apply(&mut self, message: Message) {
		let _ = self.playground.update(message);
	}
}
