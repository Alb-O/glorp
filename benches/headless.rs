#![allow(missing_docs)]

use criterion::{BatchSize, BenchmarkId, Criterion, criterion_group, criterion_main};
use iced::advanced::renderer::{Headless, Style};
use iced::mouse;
use iced::{Color, Font, Pixels, Size, Theme};
use iced_runtime::{UserInterface, user_interface};
use liney::{HeadlessScenario, HeadlessScriptScenario, Playground};
use pollster::block_on;

use std::env;
use std::hint::black_box;

criterion_group!(
	benches,
	benchmark_headless_playground,
	benchmark_headless_script_sequences
);
criterion_main!(benches);

const VIEWPORT_SCALE_FACTOR: f32 = 1.0;
const VIEWPORT_PHYSICAL_WIDTH: u32 = 1600;
const VIEWPORT_PHYSICAL_HEIGHT: u32 = 1000;

fn benchmark_headless_playground(c: &mut Criterion) {
	let mut group = c.benchmark_group("playground/headless");

	for scenario in HeadlessScenario::ALL {
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

fn benchmark_headless_script_sequences(c: &mut Criterion) {
	let mut group = c.benchmark_group("playground/headless-script");

	for scenario in HeadlessScriptScenario::ALL {
		group.bench_function(scenario.label(), |b| {
			b.iter_batched(
				|| {
					let mut playground = Playground::headless();
					playground.configure_headless_script_scenario(scenario);
					playground
				},
				|mut playground| {
					black_box(playground.run_headless_script_scenario(scenario));
				},
				BatchSize::SmallInput,
			);
		});
	}

	group.finish();
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
	fn new(scenario: HeadlessScenario) -> Self {
		let backend = env::var("LINEY_HEADLESS_BACKEND").ok();
		let renderer = block_on(<iced::Renderer as Headless>::new(
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
		let playground = Playground::headless();

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

	fn configure(&mut self, scenario: HeadlessScenario) {
		self.playground.configure_headless_scenario(scenario);
	}

	fn render_frame(&mut self) -> usize {
		let mut user_interface = UserInterface::build(
			self.playground.headless_view(),
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
}
