use {
	crate::{
		ConfigStore, ConfigStorePaths,
		events::{SubscriptionCheckpoint, SubscriptionSet},
		execute,
		state::RuntimeState,
	},
	glorp_api::{GlorpCall, GlorpCallResult, GlorpCaller, GlorpError, GlorpOutcome, SamplePreset},
	glorp_editor::{ScenePresentation, sample_preset_text},
};

#[derive(Debug, Clone)]
pub struct RuntimeOptions {
	pub paths: ConfigStorePaths,
}

pub struct GlorpRuntime {
	pub(crate) config_store: ConfigStore,
	pub(crate) state: RuntimeState,
	pub(crate) subscriptions: SubscriptionSet,
}

impl GlorpRuntime {
	pub fn new(options: RuntimeOptions) -> Result<Self, GlorpError> {
		let config_store = ConfigStore::new(options.paths);
		let config = config_store.load()?;
		let preset = config.editor.preset.unwrap_or(SamplePreset::Tall);
		let state = RuntimeState::new(config, sample_preset_text(preset));
		Ok(Self {
			config_store,
			state,
			subscriptions: SubscriptionSet::default(),
		})
	}

	pub fn subscriptions_state(&self) -> SubscriptionCheckpoint {
		self.subscriptions.checkpoint()
	}

	pub fn subscriptions(&self) -> &SubscriptionSet {
		&self.subscriptions
	}

	pub fn restore_subscriptions(&mut self, subscriptions: SubscriptionCheckpoint) {
		self.subscriptions.restore(subscriptions);
	}

	pub fn publish_changed(&mut self, outcome: &GlorpOutcome) {
		self.subscriptions.publish_changed(outcome);
	}

	pub fn gui_edit(&mut self, request: crate::GuiEditRequest) -> Result<crate::GuiEditResponse, GlorpError> {
		execute::execute_gui_edit(self, request)
	}

	pub fn gui_frame(&mut self) -> crate::GuiRuntimeFrame {
		self.gui_frame_at(crate::GuiLayoutRequest {
			layout_width: crate::DEFAULT_LAYOUT_WIDTH,
		})
	}

	pub fn gui_frame_at(&mut self, layout: crate::GuiLayoutRequest) -> crate::GuiRuntimeFrame {
		execute::sync_gui_layout(self, layout.layout_width);
		let (undo_depth, redo_depth) = self.state.session.history_depths();
		let document_text = self.state.session.text();
		let document_sync = execute::document_sync_ref(
			self.state.revisions.editor,
			document_text,
			crate::GuiDocumentSyncReason::Bootstrap,
		);
		crate::GuiRuntimeFrame {
			config: self.state.config.clone(),
			layout_width: self.state.session.layout_width(),
			revisions: self.state.revisions,
			document_text: document_sync.is_none().then(|| document_text.to_owned()),
			undo_depth,
			redo_depth,
			scene_summary: self.state.session.scene_summary(),
			document_sync,
		}
	}

	pub fn gui_document_fetch(
		&mut self, _request: crate::GuiDocumentFetchRequest,
	) -> (crate::GuiDocumentFetchResponse, String) {
		let text = self.state.session.text().to_owned();
		(
			crate::GuiDocumentFetchResponse {
				revision: self.state.revisions.editor,
				bytes: text.len(),
			},
			text,
		)
	}

	pub fn gui_scene_fetch(&mut self, request: crate::GuiSceneFetchRequest) -> crate::GuiSceneFetchResponse {
		self.gui_scene_fetch_at(request)
	}

	pub fn gui_scene_fetch_at(&mut self, request: crate::GuiSceneFetchRequest) -> crate::GuiSceneFetchResponse {
		execute::sync_gui_layout(self, request.layout.layout_width);
		let scene_summary = self.state.session.scene_summary();
		let current_width = self.state.session.layout_width();
		if request.scene_revision == scene_summary.revision
			&& (request.layout.layout_width - current_width).abs() <= f32::EPSILON
		{
			return crate::GuiSceneFetchResponse::NotModified;
		}

		let (scene, duration) = self.state.session.fetch_scene();
		if let Some(duration) = duration {
			self.state.perf.record_scene_build(duration.as_secs_f64() * 1000.0);
		}
		let bytes = execute::scene_payload_bytes(&scene);
		crate::GuiSceneFetchResponse::Payload(crate::GuiSceneFetchRef {
			scene_revision: scene.revision,
			layout_width: current_width,
			bytes,
			codec: crate::GuiPayloadCodec::Postcard,
		})
	}

	pub fn gui_scene_payload_at(&mut self, request: crate::GuiSceneFetchRequest) -> ScenePresentation {
		execute::sync_gui_layout(self, request.layout.layout_width);
		let (scene, duration) = self.state.session.fetch_scene();
		if let Some(duration) = duration {
			self.state.perf.record_scene_build(duration.as_secs_f64() * 1000.0);
		}
		scene
	}

	pub fn gui_scene_fetch_legacy(&mut self) -> ScenePresentation {
		self.gui_scene_payload_at(crate::GuiSceneFetchRequest {
			layout: crate::GuiLayoutRequest {
				layout_width: crate::DEFAULT_LAYOUT_WIDTH,
			},
			scene_revision: 0,
		})
	}

	pub fn gui_shared_delta(&self, outcome: glorp_api::GlorpOutcome) -> crate::GuiSharedDelta {
		execute::gui_shared_delta(self, outcome)
	}
}

impl GlorpCaller for GlorpRuntime {
	fn call(&mut self, call: GlorpCall) -> Result<GlorpCallResult, GlorpError> {
		execute::call(self, call)
	}
}

pub fn default_runtime_paths(repo_root: impl AsRef<std::path::Path>) -> ConfigStorePaths {
	let repo_root = repo_root.as_ref();
	ConfigStorePaths {
		durable_config_path: repo_root.join("nu/default-config.nu"),
		schema_path: repo_root.join("schema/glorp-schema.json"),
		nu_module_path: repo_root.join("nu/glorp.nu"),
		nu_completions_path: repo_root.join("nu/completions.nu"),
	}
}
