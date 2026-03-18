use {
	crate::{ConfigStore, ConfigStorePaths, events::SubscriptionSet, execute, project, state::RuntimeState},
	glorp_api::{
		GlorpCapabilities, GlorpError, GlorpEvent, GlorpExec, GlorpHost, GlorpOutcome, GlorpQuery, GlorpQueryResult,
		GlorpStreamToken, GlorpSubscription, SamplePreset,
	},
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
		let preset = config.editor.preset;
		let state = RuntimeState::new(config, sample_text(preset));
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
			GlorpQuery::Snapshot(input) => GlorpQueryResult::Snapshot(project::snapshot_from_state(
				&mut self.state,
				input.scene,
				input.include_document_text,
			)),
			GlorpQuery::DocumentText => GlorpQueryResult::DocumentText(self.state.session.text().into()),
			GlorpQuery::Selection => GlorpQueryResult::Selection(project::selection_view_from_state(&self.state)),
			GlorpQuery::InspectDetails(input) => GlorpQueryResult::InspectDetails(
				project::inspect_details_view_from_state(&mut self.state, input.target),
			),
			GlorpQuery::Perf => GlorpQueryResult::Perf(project::perf_dashboard_view_from_state(&mut self.state)),
			GlorpQuery::Ui => GlorpQueryResult::Ui(project::ui_state_view(&self.state)),
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

fn sample_text(preset: Option<SamplePreset>) -> &'static str {
	match preset.unwrap_or(SamplePreset::Tall) {
		SamplePreset::Tall => concat!(
			"chapter 01: office affine ffi ffl fj\n",
			"chapter 02: 漢字カタカナ and Latin in one lane\n",
			"chapter 03: السلام عليكم مع سطور إضافية\n",
			"chapter 04: emoji 🙂🚀👩‍💻 over baseline checks\n",
			"chapter 05: end marker"
		),
		SamplePreset::Mixed => "office affine ffi ffl\n漢字カタカナ and Latin\nالسلام عليكم\nemoji 🙂🚀👩‍💻",
		SamplePreset::Rust => "fn main() {\n    println!(\"ffi -> office -> 汉字\");\n}\n",
		SamplePreset::Ligatures => "office affine final fluff ffi ffl fj",
		SamplePreset::Arabic => "السلام عليكم\nمرحبا بالعالم",
		SamplePreset::Cjk => "漢字かなカナ\n混在テキスト with ASCII",
		SamplePreset::Emoji => "🙂🚀👩‍💻 text + emoji fallback",
		SamplePreset::Custom => "",
	}
}
