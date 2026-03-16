use {
	super::Playground,
	crate::{
		HeadlessScenario, HeadlessScriptScenario, PerfScenario,
		editor::{
			EditorEditIntent, EditorHistoryIntent, EditorIntent, EditorModeIntent, EditorMotion, EditorPointerIntent,
		},
		perf::PerfMonitor,
		types::{
			CanvasEvent, CanvasTarget, ControlsMessage, Message, SamplePreset, SidebarMessage, SidebarTab,
			ViewportMessage,
		},
	},
	iced::{Point, Size},
	std::{
		fmt::Write as _,
		sync::OnceLock,
		time::{Duration, Instant},
	},
};

const HEADLESS_VIEWPORT_SIZE: Size = Size::new(1600.0, 1000.0);
const HEADLESS_BENCH_DOCUMENT_LINES: usize = 768;
const HEADLESS_LARGE_PASTE_LINES: usize = 96;
const HEADLESS_INCREMENTAL_TYPING_STEPS: usize = 256;
const HEADLESS_INCREMENTAL_LINE_BREAK_STEPS: usize = 48;
const HEADLESS_UNDO_REDO_STEPS: usize = 48;
const HEADLESS_INSERT_POSITION_ROWS: usize = 8;
const HEADLESS_DELETE_SEED_REPEAT: usize = 24;
const HEADLESS_MOTION_SWEEP_REPEATS: usize = 48;
const HEADLESS_RESIZE_SETTLE_DELAY: Duration = Duration::from_millis(16);
const HEADLESS_RESIZE_WIDTHS: [f32; 7] = [1600.0, 1240.0, 980.0, 780.0, 1120.0, 900.0, 1360.0];
const HEADLESS_POINTER_SWEEP_POINTS: [((f32, f32), (f32, f32)); 4] = [
	((32.0, 32.0), (240.0, 32.0)),
	((48.0, 64.0), (320.0, 64.0)),
	((56.0, 96.0), (360.0, 128.0)),
	((72.0, 160.0), (300.0, 160.0)),
];
const HEADLESS_INSPECT_HOVERS: [CanvasTarget; 3] = [
	CanvasTarget::Glyph {
		run_index: 0,
		glyph_index: 0,
	},
	CanvasTarget::Glyph {
		run_index: 1,
		glyph_index: 1,
	},
	CanvasTarget::Run(2),
];
const HEADLESS_BENCH_LINE_SEEDS: [&str; 6] = [
	"office affine ffi ffl fj with mixed wrap probes",
	"漢字カタカナ and Latin share one buffer lane",
	"السلام عليكم مع سطور إضافية لاختبار الالتفاف",
	"emoji 🙂🚀👩‍💻 cluster fallback should keep layout honest",
	"Rust fn main() { println!(\"bench scene rebuild\"); }",
	"bidi mix -> abc אבג 123 and ligatures office official",
];
const ASCII_LOWERCASE: &[u8; 26] = b"abcdefghijklmnopqrstuvwxyz";

impl Playground {
	pub fn configure_headless_scenario(&mut self, scenario: HeadlessScenario) {
		self.configure_headless_viewport();

		match scenario {
			HeadlessScenario::Default => {}
			HeadlessScenario::Tall => {
				let _ = self.update(Message::Controls(ControlsMessage::LoadPreset(SamplePreset::Tall)));
			}
			HeadlessScenario::TallInspect => {
				let _ = self.update(Message::Controls(ControlsMessage::LoadPreset(SamplePreset::Tall)));
				let _ = self.update(Message::Controls(ControlsMessage::ShowHitboxesChanged(true)));
				let _ = self.update(Message::Controls(ControlsMessage::ShowBaselinesChanged(true)));
				let _ = self.update(Message::Sidebar(SidebarMessage::SelectTab(SidebarTab::Inspect)));
				let _ = self.update(Message::Canvas(CanvasEvent::Hovered(Some(CanvasTarget::Glyph {
					run_index: 0,
					glyph_index: 0,
				}))));
			}
			HeadlessScenario::TallPerf => {
				let _ = self.update(Message::Controls(ControlsMessage::LoadPreset(SamplePreset::Tall)));
				let _ = self.update(Message::Sidebar(SidebarMessage::SelectTab(SidebarTab::Perf)));
			}
		}
	}

	pub fn configure_headless_script_scenario(&mut self, scenario: HeadlessScriptScenario) {
		self.configure_headless_viewport();
		self.load_headless_document(headless_bench_document());

		match scenario {
			HeadlessScriptScenario::LargePaste
			| HeadlessScriptScenario::IncrementalTyping
			| HeadlessScriptScenario::IncrementalLineBreaks
			| HeadlessScriptScenario::UndoRedoBurst => self.position_headless_insert_point(),
			HeadlessScriptScenario::BackspaceBurst => {
				self.position_headless_insert_point();
				self.apply_headless_insert(headless_delete_seed_chunk().to_owned());
			}
			HeadlessScriptScenario::DeleteForwardBurst => {
				self.position_headless_insert_point();
				self.apply_headless_insert(headless_delete_seed_chunk().to_owned());
				self.rewind_insert_caret(delete_seed_char_count());
			}
			HeadlessScriptScenario::MotionSweep
			| HeadlessScriptScenario::PointerSelectionSweep
			| HeadlessScriptScenario::ResizeReflowSweep => {}
			HeadlessScriptScenario::InspectInteractionSweep => {
				let _ = self.update(Message::Controls(ControlsMessage::ShowHitboxesChanged(true)));
				let _ = self.update(Message::Controls(ControlsMessage::ShowBaselinesChanged(true)));
				let _ = self.update(Message::Sidebar(SidebarMessage::SelectTab(SidebarTab::Inspect)));
			}
		}
	}

	pub(crate) fn configure_headless_perf_scenario(&mut self, scenario: PerfScenario) {
		match scenario {
			PerfScenario::Default => self.configure_headless_scenario(HeadlessScenario::Default),
			PerfScenario::Tall => self.configure_headless_scenario(HeadlessScenario::Tall),
			PerfScenario::TallInspect => self.configure_headless_scenario(HeadlessScenario::TallInspect),
			PerfScenario::TallPerf => self.configure_headless_scenario(HeadlessScenario::TallPerf),
			PerfScenario::IncrementalTyping => {
				self.configure_headless_script_scenario(HeadlessScriptScenario::IncrementalTyping);
			}
			PerfScenario::MotionSweep => {
				self.configure_headless_script_scenario(HeadlessScriptScenario::MotionSweep);
			}
			PerfScenario::ResizeReflow => {
				self.configure_headless_script_scenario(HeadlessScriptScenario::ResizeReflowSweep);
				let _ = self.update(Message::Sidebar(SidebarMessage::SelectTab(SidebarTab::Inspect)));
			}
			PerfScenario::InspectInteraction => {
				self.configure_headless_script_scenario(HeadlessScriptScenario::InspectInteractionSweep);
			}
		}
	}

	pub(crate) fn run_headless_perf_step(&mut self, scenario: PerfScenario, step: usize) {
		match scenario {
			PerfScenario::Default | PerfScenario::Tall | PerfScenario::TallInspect | PerfScenario::TallPerf => {}
			PerfScenario::IncrementalTyping => self.perform_incremental_typing_step(step),
			PerfScenario::MotionSweep => self.perform_motion_sweep_step(step),
			PerfScenario::ResizeReflow => self.perform_resize_reflow_step(step),
			PerfScenario::InspectInteraction => self.perform_inspect_interaction_step(step),
		}
	}

	pub fn run_headless_script_scenario(&mut self, scenario: HeadlessScriptScenario) -> usize {
		match scenario {
			HeadlessScriptScenario::LargePaste => {
				self.apply_headless_insert(headless_large_paste_chunk().to_owned());
			}
			HeadlessScriptScenario::IncrementalTyping => {
				for step in 0..HEADLESS_INCREMENTAL_TYPING_STEPS {
					let next = char::from(ASCII_LOWERCASE[step % ASCII_LOWERCASE.len()]);
					self.apply_headless_insert(next.to_string());
				}
			}
			HeadlessScriptScenario::IncrementalLineBreaks => {
				for step in 0..HEADLESS_INCREMENTAL_LINE_BREAK_STEPS {
					self.apply_headless_insert(headless_incremental_line_break(step));
				}
			}
			HeadlessScriptScenario::UndoRedoBurst => {
				for step in 0..HEADLESS_UNDO_REDO_STEPS {
					self.apply_headless_insert(format!("u{step:02}"));
				}

				for _ in 0..HEADLESS_UNDO_REDO_STEPS {
					self.apply_headless_history(EditorHistoryIntent::Undo);
				}

				for _ in 0..HEADLESS_UNDO_REDO_STEPS {
					self.apply_headless_history(EditorHistoryIntent::Redo);
				}
			}
			HeadlessScriptScenario::BackspaceBurst => {
				for _ in 0..delete_seed_char_count() {
					self.apply_headless_edit(EditorEditIntent::Backspace);
				}
			}
			HeadlessScriptScenario::DeleteForwardBurst => {
				for _ in 0..delete_seed_char_count() {
					self.apply_headless_edit(EditorEditIntent::DeleteForward);
				}
			}
			HeadlessScriptScenario::MotionSweep => self.perform_motion_sweep(),
			HeadlessScriptScenario::PointerSelectionSweep => self.perform_pointer_selection_sweep(),
			HeadlessScriptScenario::ResizeReflowSweep => self.perform_resize_reflow_sweep(),
			HeadlessScriptScenario::InspectInteractionSweep => self.perform_inspect_interaction_sweep(),
		}

		self.headless_observation()
	}

	pub(crate) fn reset_perf_monitor(&mut self) {
		self.perf = PerfMonitor::default();
	}

	pub(crate) fn flush_perf_metrics(&mut self) {
		self.perf.flush_canvas_metrics();
	}

	pub(crate) fn perf_dashboard(&self) -> crate::perf::PerfDashboard {
		self.perf
			.dashboard(self.session.scene(), self.session.mode(), self.session.text().len())
	}

	fn configure_headless_viewport(&mut self) {
		self.perf = PerfMonitor::default();
		let _ = self.update(Message::Viewport(ViewportMessage::CanvasResized(
			HEADLESS_VIEWPORT_SIZE,
		)));
	}

	fn load_headless_document(&mut self, text: &str) {
		self.controls.preset = SamplePreset::Custom;
		self.sidebar.set_active_tab(SidebarTab::Controls);
		self.session.reset_with_preset(text, self.scene_config());
		self.sidebar.sync_after_scene_refresh();
		self.viewport.mark_scene_applied(Instant::now());
		self.viewport.finish_scene_refresh(self.session.scene(), true);
	}

	fn position_headless_insert_point(&mut self) {
		for _ in 0..HEADLESS_INSERT_POSITION_ROWS {
			self.apply_headless_motion(EditorMotion::Down);
		}

		self.apply_headless_motion(EditorMotion::LineEnd);
		let _ = self.update(Message::Editor(EditorIntent::Mode(EditorModeIntent::EnterInsertAfter)));
	}

	fn rewind_insert_caret(&mut self, steps: usize) {
		for _ in 0..steps {
			self.apply_headless_motion(EditorMotion::Left);
		}
	}

	fn apply_headless_insert(&mut self, text: String) {
		self.apply_headless_edit(EditorEditIntent::InsertText(text));
	}

	fn apply_headless_edit(&mut self, intent: EditorEditIntent) {
		let _ = self.update(Message::Editor(EditorIntent::Edit(intent)));
	}

	fn apply_headless_history(&mut self, intent: EditorHistoryIntent) {
		let _ = self.update(Message::Editor(EditorIntent::History(intent)));
	}

	fn apply_headless_motion(&mut self, intent: EditorMotion) {
		let _ = self.update(Message::Editor(EditorIntent::Motion(intent)));
	}

	fn perform_motion_sweep(&mut self) {
		for _ in 0..HEADLESS_MOTION_SWEEP_REPEATS {
			for step in 0..headless_motion_sequence().len() {
				self.perform_motion_sweep_step(step);
			}
		}
	}

	fn perform_pointer_selection_sweep(&mut self) {
		for (start, end) in HEADLESS_POINTER_SWEEP_POINTS {
			let _ = self.update(Message::Canvas(CanvasEvent::PointerSelectionStarted {
				target: Some(CanvasTarget::Run(0)),
				intent: EditorPointerIntent::Begin {
					position: Point::new(start.0, start.1),
					select_word: false,
				},
			}));
			let _ = self.update(Message::Editor(EditorIntent::Pointer(EditorPointerIntent::Drag(
				Point::new(end.0, end.1),
			))));
			let _ = self.update(Message::Editor(EditorIntent::Pointer(EditorPointerIntent::End)));
		}
	}

	fn perform_resize_reflow_sweep(&mut self) {
		for step in 0..HEADLESS_RESIZE_WIDTHS.len() {
			self.perform_resize_reflow_step(step);
		}
	}

	fn perform_inspect_interaction_sweep(&mut self) {
		let steps = HEADLESS_INSPECT_HOVERS.len() + (HEADLESS_POINTER_SWEEP_POINTS.len() * 3);
		for step in 0..steps {
			self.perform_inspect_interaction_step(step);
		}
	}

	fn perform_incremental_typing_step(&mut self, step: usize) {
		let next = char::from(ASCII_LOWERCASE[step % ASCII_LOWERCASE.len()]);
		self.apply_headless_insert(next.to_string());
	}

	fn perform_motion_sweep_step(&mut self, step: usize) {
		self.apply_headless_motion(headless_motion_sequence()[step % headless_motion_sequence().len()]);
	}

	fn perform_resize_reflow_step(&mut self, step: usize) {
		let width = HEADLESS_RESIZE_WIDTHS[(step + 1) % HEADLESS_RESIZE_WIDTHS.len()];
		let _ = self.update(Message::Viewport(ViewportMessage::CanvasResized(Size::new(
			width,
			HEADLESS_VIEWPORT_SIZE.height,
		))));
		let _ = self.update(Message::Viewport(ViewportMessage::ResizeTick(
			Instant::now() + HEADLESS_RESIZE_SETTLE_DELAY,
		)));
	}

	fn perform_inspect_interaction_step(&mut self, step: usize) {
		if step < HEADLESS_INSPECT_HOVERS.len() {
			let _ = self.update(Message::Canvas(CanvasEvent::Hovered(Some(
				HEADLESS_INSPECT_HOVERS[step],
			))));
			return;
		}

		let pointer_step = (step - HEADLESS_INSPECT_HOVERS.len()) % (HEADLESS_POINTER_SWEEP_POINTS.len() * 3);
		let point_index = pointer_step / 3;
		let phase = pointer_step % 3;
		let (start, end) = HEADLESS_POINTER_SWEEP_POINTS[point_index];

		match phase {
			0 => {
				let _ = self.update(Message::Canvas(CanvasEvent::PointerSelectionStarted {
					target: Some(CanvasTarget::Run(1)),
					intent: EditorPointerIntent::Begin {
						position: Point::new(start.0, start.1),
						select_word: false,
					},
				}));
			}
			1 => {
				let _ = self.update(Message::Editor(EditorIntent::Pointer(EditorPointerIntent::Drag(
					Point::new(end.0, end.1),
				))));
			}
			_ => {
				let _ = self.update(Message::Editor(EditorIntent::Pointer(EditorPointerIntent::End)));
			}
		}
	}

	fn headless_observation(&self) -> usize {
		let view = self.session.view_state();
		let selection_end = view.selection.as_ref().map_or(0, |selection| selection.end);
		let selection_head = view.selection_head.unwrap_or(0);

		fold_bytes_to_usize(
			self.session
				.text()
				.len()
				.to_ne_bytes()
				.into_iter()
				.chain(selection_end.to_ne_bytes())
				.chain(selection_head.to_ne_bytes())
				.chain(self.viewport.canvas_scroll.x.max(0.0).to_bits().to_ne_bytes())
				.chain(self.viewport.canvas_scroll.y.max(0.0).to_bits().to_ne_bytes())
				.chain(self.viewport.layout_width.round().to_bits().to_ne_bytes())
				.chain(self.viewport.scene_revision.to_ne_bytes()),
		)
	}
}

fn fold_bytes_to_usize(bytes: impl IntoIterator<Item = u8>) -> usize {
	let mut hash = 0usize;
	for byte in bytes {
		hash = hash.rotate_left(5) ^ usize::from(byte);
	}
	hash
}

fn headless_bench_document() -> &'static str {
	static DOCUMENT: OnceLock<String> = OnceLock::new();

	DOCUMENT.get_or_init(|| build_headless_bench_document(HEADLESS_BENCH_DOCUMENT_LINES))
}

fn headless_large_paste_chunk() -> &'static str {
	static CHUNK: OnceLock<String> = OnceLock::new();

	CHUNK.get_or_init(|| build_headless_paste_chunk(HEADLESS_LARGE_PASTE_LINES))
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

fn headless_incremental_line_break(step: usize) -> String {
	format!("\nbranch {step:04}: line break typing probe ffi 漢字")
}

fn delete_seed_char_count() -> usize {
	headless_delete_seed_chunk().chars().count()
}

fn headless_motion_sequence() -> [EditorMotion; 6] {
	[
		EditorMotion::Down,
		EditorMotion::Right,
		EditorMotion::LineEnd,
		EditorMotion::Up,
		EditorMotion::LineStart,
		EditorMotion::Left,
	]
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
