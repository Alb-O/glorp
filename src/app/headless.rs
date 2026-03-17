use {
	super::{AppModel, EditorApp, update::AppCommand},
	crate::{
		HeadlessScenario, HeadlessScriptScenario, PerfScenario,
		editor::{EditorEditIntent, EditorIntent, EditorModeIntent, EditorMotion, EditorPointerIntent},
		perf::{PerfDashboard, PerfFramePacingSummary, PerfGraphSeries, PerfMonitor, PerfOverview, PerfRecentActivity},
		types::{CanvasTarget, ControlsMessage, SamplePreset, SidebarTab},
	},
	iced::{Point, Size},
	std::{fmt::Write as _, sync::OnceLock, time::Duration},
};

#[cfg(test)]
use crate::editor::EditorHistoryIntent;

const HEADLESS_VIEWPORT_SIZE: Size = Size::new(1600.0, 1000.0);
const HEADLESS_BENCH_DOCUMENT_LINES: usize = 768;
const HEADLESS_LINE_BREAK_DOCUMENT_LINES: usize = 128;
#[cfg(test)]
const HEADLESS_LARGE_PASTE_LINES: usize = 96;
#[cfg(test)]
const HEADLESS_INCREMENTAL_TYPING_STEPS: usize = 256;
#[cfg(test)]
const HEADLESS_INCREMENTAL_LINE_BREAK_STEPS: usize = 48;
#[cfg(test)]
const HEADLESS_UNDO_REDO_STEPS: usize = 48;
const HEADLESS_INSERT_POSITION_ROWS: usize = 8;
const HEADLESS_DELETE_SEED_REPEAT: usize = 24;
#[cfg(test)]
const HEADLESS_MOTION_SWEEP_REPEATS: usize = 48;
const HEADLESS_RESIZE_PROGRESS: [f32; 4] = [0.25, 0.5, 0.75, 1.0];
const HEADLESS_RESIZE_WIDTHS: [f32; 7] = [1600.0, 1240.0, 980.0, 780.0, 1120.0, 900.0, 1360.0];
const HEADLESS_MOTION_SEQUENCE: [EditorMotion; 6] = [
	EditorMotion::Down,
	EditorMotion::Right,
	EditorMotion::LineEnd,
	EditorMotion::Up,
	EditorMotion::LineStart,
	EditorMotion::Left,
];
const HEADLESS_POINTER_SWEEP_POINTS: [((f32, f32), (f32, f32)); 4] = [
	((32.0, 32.0), (240.0, 32.0)),
	((48.0, 64.0), (320.0, 64.0)),
	((56.0, 96.0), (360.0, 128.0)),
	((72.0, 160.0), (300.0, 160.0)),
];
const HEADLESS_INSPECT_HOVERS: [CanvasTarget; 3] =
	[CanvasTarget::Cluster(0), CanvasTarget::Cluster(1), CanvasTarget::Run(2)];
const HEADLESS_BENCH_LINE_SEEDS: [&str; 6] = [
	"office affine ffi ffl fj with mixed wrap probes",
	"漢字カタカナ and Latin share one buffer lane",
	"السلام عليكم مع سطور إضافية لاختبار الالتفاف",
	"emoji 🙂🚀👩‍💻 cluster fallback should keep layout honest",
	"Rust fn main() { println!(\"bench scene rebuild\"); }",
	"bidi mix -> abc אבג 123 and ligatures office official",
];
const ASCII_LOWERCASE: &[u8; 26] = b"abcdefghijklmnopqrstuvwxyz";

pub(crate) struct HeadlessDriver<'a> {
	app: &'a mut AppModel,
}

impl EditorApp {
	pub(crate) fn headless_driver(&mut self) -> HeadlessDriver<'_> {
		HeadlessDriver { app: &mut self.model }
	}

	pub(crate) fn reset_perf_monitor(&mut self) {
		self.model.perf = PerfMonitor::default();
	}

	pub(crate) fn record_headless_ui_build(&mut self, duration: Duration) {
		self.model.perf.record_ui_build(duration);
	}

	pub(crate) fn record_headless_ui_draw(&mut self, duration: Duration) {
		self.model.perf.record_ui_draw(duration);
	}

	pub(crate) fn flush_perf_metrics(&mut self) {
		self.model.perf.flush_canvas_metrics();
	}

	pub(crate) fn perf_dashboard(&mut self) -> PerfDashboard {
		self.model.ensure_scene_ready();
		let snapshot = self.model.session.snapshot();

		snapshot.scene.as_ref().map_or_else(
			|| {
				unavailable_perf_dashboard(
					snapshot.mode(),
					snapshot.editor_bytes(),
					self.model.viewport.layout_width,
				)
			},
			|scene| {
				self.model.perf.dashboard(
					scene.layout.as_ref(),
					self.model.session.mode(),
					snapshot.editor.editor_bytes(),
				)
			},
		)
	}
}

impl HeadlessDriver<'_> {
	pub(crate) fn configure_scenario(&mut self, scenario: HeadlessScenario) {
		self.configure_viewport();

		match scenario {
			HeadlessScenario::Default => {}
			HeadlessScenario::Tall => {
				self.dispatch_command(AppCommand::Control(ControlsMessage::LoadPreset(SamplePreset::Tall)))
			}
			HeadlessScenario::TallInspect => {
				self.dispatch_command(AppCommand::Control(ControlsMessage::LoadPreset(SamplePreset::Tall)));
				self.enable_inspect_mode();
				self.dispatch_command(AppCommand::HoverCanvas(Some(CanvasTarget::Cluster(0))));
			}
			HeadlessScenario::TallPerf => {
				self.dispatch_command(AppCommand::Control(ControlsMessage::LoadPreset(SamplePreset::Tall)));
				self.dispatch_command(AppCommand::SelectSidebarTab(SidebarTab::Perf));
			}
		}
	}

	pub(crate) fn configure_script_scenario(&mut self, scenario: HeadlessScriptScenario) {
		self.configure_viewport();
		self.load_document(match scenario {
			HeadlessScriptScenario::IncrementalLineBreaks => headless_line_break_document(),
			_ => headless_bench_document(),
		});

		match scenario {
			HeadlessScriptScenario::LargePaste
			| HeadlessScriptScenario::IncrementalTyping
			| HeadlessScriptScenario::IncrementalLineBreaks
			| HeadlessScriptScenario::UndoRedoBurst => self.position_insert_point(),
			HeadlessScriptScenario::BackspaceBurst => {
				self.position_insert_point();
				self.apply_insert(headless_delete_seed_chunk());
			}
			HeadlessScriptScenario::DeleteForwardBurst => {
				self.position_insert_point();
				self.apply_insert(headless_delete_seed_chunk());
				self.rewind_insert_caret(delete_seed_char_count());
			}
			HeadlessScriptScenario::MotionSweep
			| HeadlessScriptScenario::PointerSelectionSweep
			| HeadlessScriptScenario::ResizeReflowSweep => {}
			HeadlessScriptScenario::InspectInteractionSweep => {
				self.enable_inspect_mode();
			}
		}
	}

	pub(crate) fn configure_perf_scenario(&mut self, scenario: PerfScenario) {
		match scenario {
			PerfScenario::Default | PerfScenario::Tall | PerfScenario::TallInspect | PerfScenario::TallPerf => {
				self.configure_scenario(match scenario {
					PerfScenario::Default => HeadlessScenario::Default,
					PerfScenario::Tall => HeadlessScenario::Tall,
					PerfScenario::TallInspect => HeadlessScenario::TallInspect,
					PerfScenario::TallPerf => HeadlessScenario::TallPerf,
					_ => unreachable!("handled by the outer match"),
				});
			}
			PerfScenario::IncrementalTyping | PerfScenario::MotionSweep | PerfScenario::InspectInteraction => self
				.configure_script_scenario(match scenario {
					PerfScenario::IncrementalTyping => HeadlessScriptScenario::IncrementalTyping,
					PerfScenario::MotionSweep => HeadlessScriptScenario::MotionSweep,
					PerfScenario::InspectInteraction => HeadlessScriptScenario::InspectInteractionSweep,
					_ => unreachable!("handled by the outer match"),
				}),
			PerfScenario::ResizeReflow => {
				self.configure_script_scenario(HeadlessScriptScenario::ResizeReflowSweep);
				self.dispatch_command(AppCommand::SelectSidebarTab(SidebarTab::Inspect));
			}
		}
	}

	pub(crate) fn run_perf_step(&mut self, scenario: PerfScenario, step: usize) {
		match scenario {
			PerfScenario::Default | PerfScenario::Tall | PerfScenario::TallInspect | PerfScenario::TallPerf => {}
			PerfScenario::IncrementalTyping => self.perform_incremental_typing_step(step),
			PerfScenario::MotionSweep => self.perform_motion_sweep_step(step),
			PerfScenario::ResizeReflow => self.perform_resize_reflow_step(step),
			PerfScenario::InspectInteraction => self.perform_inspect_interaction_step(step),
		}
	}

	#[cfg(test)]
	pub(crate) fn run_script_scenario(&mut self, scenario: HeadlessScriptScenario) -> usize {
		match scenario {
			HeadlessScriptScenario::LargePaste => self.apply_insert(headless_large_paste_chunk()),
			HeadlessScriptScenario::IncrementalTyping => {
				for step in 0..HEADLESS_INCREMENTAL_TYPING_STEPS {
					self.apply_insert(headless_incremental_typing_char(step).to_string());
				}
			}
			HeadlessScriptScenario::IncrementalLineBreaks => {
				for step in 0..HEADLESS_INCREMENTAL_LINE_BREAK_STEPS {
					self.apply_insert(headless_incremental_line_break(step));
				}
			}
			HeadlessScriptScenario::UndoRedoBurst => {
				for step in 0..HEADLESS_UNDO_REDO_STEPS {
					self.apply_insert(format!("u{step:02}"));
				}

				self.repeat_history(EditorHistoryIntent::Undo, HEADLESS_UNDO_REDO_STEPS);
				self.repeat_history(EditorHistoryIntent::Redo, HEADLESS_UNDO_REDO_STEPS);
			}
			HeadlessScriptScenario::BackspaceBurst | HeadlessScriptScenario::DeleteForwardBurst => {
				self.repeat_edit(&delete_burst_intent(scenario), delete_seed_char_count())
			}
			HeadlessScriptScenario::MotionSweep => self.perform_motion_sweep(),
			HeadlessScriptScenario::PointerSelectionSweep => self.perform_pointer_selection_sweep(),
			HeadlessScriptScenario::ResizeReflowSweep => self.perform_resize_reflow_sweep(),
			HeadlessScriptScenario::InspectInteractionSweep => self.perform_inspect_interaction_sweep(),
		}

		self.observation()
	}

	fn configure_viewport(&mut self) {
		self.app.perf = PerfMonitor::default();
		self.dispatch_command(AppCommand::ObserveCanvasResize(HEADLESS_VIEWPORT_SIZE));
	}

	fn enable_inspect_mode(&mut self) {
		self.dispatch_command(AppCommand::Control(ControlsMessage::ShowHitboxesChanged(true)));
		self.dispatch_command(AppCommand::Control(ControlsMessage::ShowBaselinesChanged(true)));
		self.dispatch_command(AppCommand::SelectSidebarTab(SidebarTab::Inspect));
	}

	fn load_document(&mut self, text: &str) {
		self.dispatch_command(AppCommand::SelectSidebarTab(SidebarTab::Controls));
		self.dispatch_command(AppCommand::ReplaceDocument(text.to_string()));
	}

	fn position_insert_point(&mut self) {
		self.repeat_motion(EditorMotion::Down, HEADLESS_INSERT_POSITION_ROWS);
		self.apply_motion(EditorMotion::LineEnd);
		self.dispatch_command(AppCommand::editor(EditorIntent::Mode(
			EditorModeIntent::EnterInsertAfter,
		)));
	}

	fn rewind_insert_caret(&mut self, steps: usize) {
		self.repeat_motion(EditorMotion::Left, steps);
	}

	fn apply_insert(&mut self, text: impl Into<String>) {
		self.apply_edit(EditorEditIntent::InsertText(text.into()));
	}

	fn apply_edit(&mut self, intent: EditorEditIntent) {
		self.dispatch_command(AppCommand::editor(EditorIntent::Edit(intent)));
	}

	#[cfg(test)]
	fn apply_history(&mut self, intent: EditorHistoryIntent) {
		self.dispatch_command(AppCommand::editor(EditorIntent::History(intent)));
	}

	fn apply_motion(&mut self, intent: EditorMotion) {
		self.dispatch_command(AppCommand::editor(EditorIntent::Motion(intent)));
	}

	#[cfg(test)]
	fn repeat_edit(&mut self, intent: &EditorEditIntent, steps: usize) {
		for _ in 0..steps {
			self.apply_edit(intent.clone());
		}
	}

	#[cfg(test)]
	fn repeat_history(&mut self, intent: EditorHistoryIntent, steps: usize) {
		for _ in 0..steps {
			self.apply_history(intent);
		}
	}

	fn repeat_motion(&mut self, intent: EditorMotion, steps: usize) {
		for _ in 0..steps {
			self.apply_motion(intent);
		}
	}

	#[cfg(test)]
	fn perform_motion_sweep(&mut self) {
		for _ in 0..HEADLESS_MOTION_SWEEP_REPEATS {
			for motion in HEADLESS_MOTION_SEQUENCE {
				self.apply_motion(motion);
			}
		}
	}

	#[cfg(test)]
	fn perform_pointer_selection_sweep(&mut self) {
		for (start, end) in HEADLESS_POINTER_SWEEP_POINTS {
			self.begin_pointer_selection(CanvasTarget::Run(0), start);
			self.drag_pointer_selection(end);
			self.end_pointer_selection();
		}
	}

	#[cfg(test)]
	fn perform_resize_reflow_sweep(&mut self) {
		for step in 0..HEADLESS_RESIZE_WIDTHS.len() {
			self.perform_resize_reflow_step(step);
		}
	}

	#[cfg(test)]
	fn perform_inspect_interaction_sweep(&mut self) {
		let steps = HEADLESS_INSPECT_HOVERS.len() + (HEADLESS_POINTER_SWEEP_POINTS.len() * 3);
		for step in 0..steps {
			self.perform_inspect_interaction_step(step);
		}
	}

	fn perform_incremental_typing_step(&mut self, step: usize) {
		self.apply_insert(headless_incremental_typing_char(step).to_string());
	}

	fn perform_motion_sweep_step(&mut self, step: usize) {
		self.apply_motion(HEADLESS_MOTION_SEQUENCE[step % HEADLESS_MOTION_SEQUENCE.len()]);
	}

	fn perform_resize_reflow_step(&mut self, step: usize) {
		let width_count = HEADLESS_RESIZE_WIDTHS.len();
		let start = HEADLESS_RESIZE_WIDTHS[step % width_count];
		let target = HEADLESS_RESIZE_WIDTHS[(step + 1) % width_count];

		for progress in HEADLESS_RESIZE_PROGRESS {
			let width = (target - start).mul_add(progress, start);
			self.dispatch_command(AppCommand::ObserveCanvasResize(Size::new(
				width,
				HEADLESS_VIEWPORT_SIZE.height,
			)));
		}

		self.dispatch_command(AppCommand::FlushResizeReflow);
	}

	fn perform_inspect_interaction_step(&mut self, step: usize) {
		if step < HEADLESS_INSPECT_HOVERS.len() {
			self.dispatch_command(AppCommand::HoverCanvas(Some(HEADLESS_INSPECT_HOVERS[step])));
			return;
		}

		let pointer_step = (step - HEADLESS_INSPECT_HOVERS.len()) % (HEADLESS_POINTER_SWEEP_POINTS.len() * 3);
		let point_index = pointer_step / 3;
		let phase = pointer_step % 3;
		let (start, end) = HEADLESS_POINTER_SWEEP_POINTS[point_index];

		match phase {
			0 => self.begin_pointer_selection(CanvasTarget::Run(1), start),
			1 => self.drag_pointer_selection(end),
			_ => self.end_pointer_selection(),
		}
	}

	#[cfg(test)]
	fn observation(&self) -> usize {
		let snapshot = self.app.session.snapshot();
		let view = &snapshot.editor.editor;
		let selection_end = view.selection.as_ref().map_or(0, |selection| selection.end);
		let selection_head = view.selection_head.unwrap_or(0);
		let scene_revision = snapshot.scene.as_ref().map_or(0, |scene| scene.revision);

		fold_bytes_to_usize(
			self.app
				.session
				.text()
				.len()
				.to_ne_bytes()
				.into_iter()
				.chain(selection_end.to_ne_bytes())
				.chain(selection_head.to_ne_bytes())
				.chain(self.app.viewport.canvas_scroll.x.max(0.0).to_bits().to_ne_bytes())
				.chain(self.app.viewport.canvas_scroll.y.max(0.0).to_bits().to_ne_bytes())
				.chain(self.app.viewport.layout_width.round().to_bits().to_ne_bytes())
				.chain(scene_revision.to_ne_bytes()),
		)
	}

	fn dispatch_command(&mut self, command: AppCommand) {
		let _ = self.app.perform(command);
	}

	fn begin_pointer_selection(&mut self, target: CanvasTarget, position: (f32, f32)) {
		self.dispatch_command(AppCommand::BeginPointerSelection {
			target: Some(target),
			intent: EditorPointerIntent::Begin {
				position: Point::new(position.0, position.1),
				select_word: false,
			},
		});
	}

	fn drag_pointer_selection(&mut self, position: (f32, f32)) {
		self.dispatch_command(AppCommand::editor(EditorIntent::Pointer(EditorPointerIntent::Drag(
			Point::new(position.0, position.1),
		))));
	}

	fn end_pointer_selection(&mut self) {
		self.dispatch_command(AppCommand::editor(EditorIntent::Pointer(EditorPointerIntent::End)));
	}
}

fn unavailable_perf_dashboard(
	editor_mode: crate::editor::EditorMode, editor_bytes: usize, layout_width: f32,
) -> PerfDashboard {
	PerfDashboard {
		overview: PerfOverview {
			editor_mode,
			editor_bytes,
			editor_chars: 0,
			line_count: 0,
			run_count: 0,
			glyph_count: 0,
			cluster_count: 0,
			font_count: 0,
			warning_count: 0,
			scene_width: 0.0,
			scene_height: 0.0,
			layout_width,
		},
		hot_paths: Vec::new(),
		recent_activity: vec![PerfRecentActivity {
			label: "scene",
			recent_ms: std::sync::Arc::from([]),
		}],
		frame_pacing: PerfFramePacingSummary {
			fps: 0.0,
			last_ms: 0.0,
			avg_ms: 0.0,
			max_ms: 0.0,
			total_draws: 0,
			over_budget: 0,
			severe_jank: 0,
			cache_hits: 0,
			cache_misses: 0,
			recent_ms: std::sync::Arc::from([]),
		},
		graphs: vec![PerfGraphSeries {
			title: "scene",
			samples_ms: std::sync::Arc::from([]),
			ceiling_ms: 1.0,
			latest_ms: 0.0,
			avg_ms: 0.0,
			p95_ms: 0.0,
			warning_ms: None,
			severe_ms: None,
		}],
	}
}

#[cfg(test)]
fn fold_bytes_to_usize(bytes: impl IntoIterator<Item = u8>) -> usize {
	bytes
		.into_iter()
		.fold(0usize, |hash, byte| hash.rotate_left(5) ^ usize::from(byte))
}

fn headless_bench_document() -> &'static str {
	static DOCUMENT: OnceLock<String> = OnceLock::new();

	DOCUMENT.get_or_init(|| build_headless_bench_document(HEADLESS_BENCH_DOCUMENT_LINES))
}

#[cfg(test)]
fn headless_large_paste_chunk() -> &'static str {
	static CHUNK: OnceLock<String> = OnceLock::new();

	CHUNK.get_or_init(|| build_headless_paste_chunk(HEADLESS_LARGE_PASTE_LINES))
}

fn headless_line_break_document() -> &'static str {
	static DOCUMENT: OnceLock<String> = OnceLock::new();

	DOCUMENT.get_or_init(|| build_headless_bench_document(HEADLESS_LINE_BREAK_DOCUMENT_LINES))
}

fn headless_delete_seed_chunk() -> &'static str {
	static CHUNK: OnceLock<String> = OnceLock::new();

	CHUNK.get_or_init(build_headless_delete_seed_chunk)
}

fn build_headless_bench_document(lines: usize) -> String {
	let mut text = String::with_capacity(lines * 96);

	for line in 0..lines {
		let seed = HEADLESS_BENCH_LINE_SEEDS[line % HEADLESS_BENCH_LINE_SEEDS.len()];
		let _ = writeln!(&mut text, "section {line:04}: {seed}");
	}

	text
}

#[cfg(test)]
fn build_headless_paste_chunk(lines: usize) -> String {
	let mut text = String::with_capacity(lines * 72);

	for line in 0..lines {
		let _ = writeln!(
			&mut text,
			"paste {line:04}: incremental bench payload ffi 漢字 🙂 wrap probe"
		);
	}

	text
}

fn build_headless_delete_seed_chunk() -> String {
	"delete-forward-burst ".repeat(HEADLESS_DELETE_SEED_REPEAT)
}

fn headless_incremental_typing_char(step: usize) -> char {
	char::from(ASCII_LOWERCASE[step % ASCII_LOWERCASE.len()])
}

#[cfg(test)]
fn headless_incremental_line_break(step: usize) -> String {
	format!("\nbranch {step:04}: line break typing probe ffi 漢字")
}

#[cfg(test)]
fn delete_burst_intent(scenario: HeadlessScriptScenario) -> EditorEditIntent {
	match scenario {
		HeadlessScriptScenario::BackspaceBurst => EditorEditIntent::Backspace,
		HeadlessScriptScenario::DeleteForwardBurst => EditorEditIntent::DeleteForward,
		_ => unreachable!("only delete-burst scenarios should reach this helper"),
	}
}

fn delete_seed_char_count() -> usize {
	static COUNT: OnceLock<usize> = OnceLock::new();

	*COUNT.get_or_init(|| headless_delete_seed_chunk().chars().count())
}

#[cfg(test)]
pub(super) fn headless_large_paste_chunk_len() -> usize {
	headless_large_paste_chunk().len()
}

#[cfg(test)]
pub(super) const fn headless_incremental_typing_steps() -> usize {
	HEADLESS_INCREMENTAL_TYPING_STEPS
}

#[cfg(test)]
pub(super) const fn headless_incremental_line_break_steps() -> usize {
	HEADLESS_INCREMENTAL_LINE_BREAK_STEPS
}

#[cfg(test)]
pub(super) const fn headless_undo_redo_steps() -> usize {
	HEADLESS_UNDO_REDO_STEPS
}

#[cfg(test)]
pub(super) fn headless_delete_seed_char_count() -> usize {
	delete_seed_char_count()
}
