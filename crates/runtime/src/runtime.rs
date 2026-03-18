use {
	crate::{ConfigStore, ConfigStorePaths, events::SubscriptionSet, execute, state::RuntimeState},
	glorp_api::{GlorpCall, GlorpCallResult, GlorpCaller, GlorpError, GlorpOutcome, SamplePreset},
	glorp_editor::sample_preset_text,
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

	pub fn subscriptions_state(&self) -> SubscriptionSet {
		self.subscriptions.clone()
	}

	pub fn restore_subscriptions(&mut self, subscriptions: SubscriptionSet) {
		self.subscriptions = subscriptions;
	}

	pub fn publish_changed(&mut self, outcome: &GlorpOutcome) {
		self.subscriptions.publish_changed(outcome);
	}

	pub fn execute_gui(&mut self, command: crate::GuiCommand) -> Result<(), GlorpError> {
		execute::execute_gui(self, command)
	}

	pub fn gui_frame(&mut self) -> crate::GuiRuntimeFrame {
		crate::GuiRuntimeFrame {
			config: self.state.config.clone(),
			ui: self.state.ui.clone(),
			revisions: self.state.revisions,
			snapshot: self.state.session.snapshot().clone(),
			document_text: self.state.session.text().into(),
		}
	}

	pub fn gui_transport_frame(&mut self) -> crate::GuiTransportFrame {
		let snapshot = self.state.session.snapshot();
		let editor = &snapshot.editor;

		crate::GuiTransportFrame {
			config: self.state.config.clone(),
			ui: self.state.ui.clone(),
			revisions: self.state.revisions,
			snapshot: crate::GuiSnapshot {
				editor: crate::GuiEditorPresentation {
					revision: editor.revision,
					viewport_metrics: editor.viewport_metrics,
					editor: editor.editor.clone(),
					editor_bytes: editor.editor_bytes,
					undo_depth: editor.undo_depth,
					redo_depth: editor.redo_depth,
				},
				scene: snapshot.scene.clone(),
			},
			document_text: self.state.session.text().into(),
		}
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
