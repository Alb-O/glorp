use {
	crate::{ConfigStore, ConfigStorePaths, events::SubscriptionSet, execute, project, state::RuntimeState},
	glorp_api::{
		GlorpCapabilities, GlorpError, GlorpEvent, GlorpExec, GlorpHost, GlorpOutcome, GlorpQuery, GlorpQueryResult,
		GlorpStreamToken, GlorpSubscription, SamplePreset,
	},
	glorp_editor::sample_preset_text,
	std::path::PathBuf,
};

#[derive(Debug, Clone)]
pub struct RuntimeOptions {
	pub paths: ConfigStorePaths,
}

pub struct GlorpRuntime {
	pub(crate) config_store: ConfigStore,
	pub(crate) state: RuntimeState,
	subscriptions: SubscriptionSet,
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
}

impl GlorpHost for GlorpRuntime {
	fn execute(&mut self, exec: GlorpExec) -> Result<GlorpOutcome, GlorpError> {
		execute::execute(self, exec)
	}

	fn query(&mut self, query: GlorpQuery) -> Result<GlorpQueryResult, GlorpError> {
		Ok(match query {
			GlorpQuery::Schema => GlorpQueryResult::Schema(glorp_api::glorp_schema()),
			GlorpQuery::Config => GlorpQueryResult::Config(self.state.config.clone()),
			GlorpQuery::DocumentText => GlorpQueryResult::DocumentText(self.state.session.text().into()),
			GlorpQuery::Editor => GlorpQueryResult::Editor(project::editor_view_from_state(&self.state)),
			GlorpQuery::Capabilities => GlorpQueryResult::Capabilities(GlorpCapabilities {
				transactions: true,
				subscriptions: true,
				transports: vec!["local".into(), "ipc".into()],
			}),
		})
	}

	fn subscribe(&mut self, request: GlorpSubscription) -> Result<GlorpStreamToken, GlorpError> {
		Ok(self.subscriptions.subscribe(request))
	}

	fn next_event(&mut self, token: GlorpStreamToken) -> Result<Option<GlorpEvent>, GlorpError> {
		self.subscriptions.next_event(token)
	}

	fn unsubscribe(&mut self, token: GlorpStreamToken) -> Result<(), GlorpError> {
		self.subscriptions.unsubscribe(token)
	}
}

pub fn default_runtime_paths(repo_root: impl Into<PathBuf>) -> ConfigStorePaths {
	let repo_root = repo_root.into();
	ConfigStorePaths {
		durable_config_path: repo_root.join("nu/default-config.nu"),
		schema_path: repo_root.join("schema/glorp-schema.json"),
		nu_module_path: repo_root.join("nu/glorp.nu"),
		nu_completions_path: repo_root.join("nu/completions.nu"),
	}
}
