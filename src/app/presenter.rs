use {
	super::{
		sidebar_cache::InspectSidebarArgs,
		sidebar_data::{InspectSidebarData, SidebarBodyData},
		store::AppStore,
	},
	crate::{
		overlay::OverlayPrimitive,
		perf::{CanvasPerfSink, unavailable_dashboard},
		presentation::SessionSnapshot,
		types::SidebarTab,
		ui::ControlsTabProps,
	},
	iced::Vector,
	std::sync::Arc,
};

#[derive(Debug, Clone)]
pub(super) struct SidebarViewModel {
	pub(super) active_tab: SidebarTab,
	pub(super) editor_mode: crate::editor::EditorMode,
	pub(super) editor_bytes: usize,
	pub(super) undo_depth: usize,
	pub(super) redo_depth: usize,
	pub(super) body: SidebarBodyData,
}

#[derive(Debug, Clone)]
pub(super) struct AppViewModel {
	pub(super) snapshot: Arc<SessionSnapshot>,
	pub(super) layout_width: f32,
	pub(super) decorations: crate::ui::CanvasDecorations,
	pub(super) inspect_overlays: Arc<[OverlayPrimitive]>,
	pub(super) inspect_targets_active: bool,
	pub(super) focused: bool,
	pub(super) scroll: Vector,
	pub(super) perf: CanvasPerfSink,
	pub(super) sidebar: SidebarViewModel,
}

pub(super) fn present(store: &AppStore) -> AppViewModel {
	let snapshot = Arc::new(store.session.snapshot().clone());
	let inspect_targets_active = store.state.sidebar.active_tab == SidebarTab::Inspect;

	AppViewModel {
		inspect_overlays: inspect_overlays(store, snapshot.as_ref(), inspect_targets_active),
		sidebar: SidebarViewModel {
			active_tab: store.state.sidebar.active_tab,
			editor_mode: snapshot.mode(),
			editor_bytes: snapshot.editor_bytes(),
			undo_depth: snapshot.editor.undo_depth,
			redo_depth: snapshot.editor.redo_depth,
			body: sidebar_body_data(store, snapshot.as_ref()),
		},
		snapshot,
		layout_width: store.state.viewport.layout_width,
		decorations: crate::ui::CanvasDecorations {
			show_baselines: store.state.controls.show_baselines,
			show_hitboxes: store.state.controls.show_hitboxes,
		},
		inspect_targets_active,
		focused: store.state.viewport.canvas_focused,
		scroll: store.state.viewport.canvas_scroll,
		perf: store.perf.sink(),
	}
}

fn sidebar_body_data(store: &AppStore, snapshot: &SessionSnapshot) -> SidebarBodyData {
	match store.state.sidebar.active_tab {
		SidebarTab::Controls => SidebarBodyData::Controls(ControlsTabProps {
			preset: store.state.controls.preset,
			font: store.state.controls.font,
			shaping: store.state.controls.shaping,
			wrapping: store.state.controls.wrapping,
			font_size: store.state.controls.font_size,
			line_height: store.state.controls.line_height,
			show_baselines: store.state.controls.show_baselines,
			show_hitboxes: store.state.controls.show_hitboxes,
		}),
		SidebarTab::Inspect => snapshot.scene.as_ref().map_or_else(
			|| SidebarBodyData::Inspect(Arc::new(unavailable_inspect_sidebar_data())),
			|scene| {
				SidebarBodyData::Inspect(store.sidebar_cache.inspect_data(InspectSidebarArgs {
					editor: &snapshot.editor,
					scene,
					text: store.session.text(),
					hovered_target: store.state.sidebar.hovered_target,
					selected_target: store.state.sidebar.selected_target,
				}))
			},
		),
		SidebarTab::Perf => snapshot.scene.as_ref().map_or_else(
			|| {
				SidebarBodyData::Perf(Arc::new(unavailable_dashboard(
					snapshot.mode(),
					snapshot.editor_bytes(),
					store.state.viewport.layout_width,
				)))
			},
			|scene| SidebarBodyData::Perf(store.sidebar_cache.perf_dashboard(&snapshot.editor, scene, &store.perf)),
		),
	}
}

fn inspect_overlays(store: &AppStore, snapshot: &SessionSnapshot, active: bool) -> Arc<[OverlayPrimitive]> {
	snapshot.scene.as_ref().filter(|_| active).map_or_else(
		|| Arc::from([]),
		|scene| {
			scene.layout.inspect_overlay_primitives(
				store.state.sidebar.hovered_target,
				store.state.sidebar.selected_target,
				store.state.viewport.layout_width,
				store.state.controls.show_hitboxes,
			)
		},
	)
}

fn unavailable_inspect_sidebar_data() -> InspectSidebarData {
	InspectSidebarData {
		warnings: Arc::from([]),
		interaction_details: Arc::<str>::from("derived scene unavailable"),
	}
}
