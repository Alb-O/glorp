mod events;
mod execute;
mod project;
mod state;

use {
	self::{
		events::{GuiSubscriptionSet, SubscriptionCheckpoint, SubscriptionSet},
		state::RuntimeState,
	},
	crate::config::{ConfigStore, ConfigStorePaths},
	glorp_api::{GlorpCall, GlorpCallResult, GlorpCaller, GlorpError, GlorpOutcome, SamplePreset},
	glorp_editor::sample_preset_text,
};

pub use self::{GlorpRuntime as RuntimeHost, state::DEFAULT_LAYOUT_WIDTH};

#[derive(Debug, Clone)]
pub struct RuntimeOptions {
	pub paths: ConfigStorePaths,
}

pub struct GlorpRuntime {
	pub(crate) config_store: ConfigStore,
	pub(crate) state: RuntimeState,
	pub(crate) subscriptions: SubscriptionSet,
	pub(crate) gui_subscriptions: GuiSubscriptionSet,
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
			gui_subscriptions: GuiSubscriptionSet::default(),
		})
	}

	pub fn subscriptions_state(&self) -> SubscriptionCheckpoint {
		self.subscriptions.checkpoint()
	}

	pub fn subscriptions(&self) -> &SubscriptionSet {
		&self.subscriptions
	}

	pub fn gui_subscriptions(&self) -> &GuiSubscriptionSet {
		&self.gui_subscriptions
	}

	pub fn restore_subscriptions(&mut self, subscriptions: SubscriptionCheckpoint) {
		self.subscriptions.restore(subscriptions);
	}

	pub fn publish_changed(&mut self, outcome: &GlorpOutcome) {
		self.subscriptions.publish_changed(outcome);
	}

	pub fn publish_gui_changed(&mut self, delta: &crate::GuiSharedDelta) {
		self.gui_subscriptions.publish_changed(delta);
	}

	pub fn gui_edit(&mut self, request: crate::GuiEditRequest) -> Result<crate::GuiEditResponse, GlorpError> {
		execute::execute_gui_edit(self, request)
	}

	pub fn gui_frame(&mut self) -> crate::GuiRuntimeFrame {
		let document_text = self.state.session.text();
		let (undo_depth, redo_depth) = self.state.session.history_depths();
		let document_sync = execute::document_sync_ref(
			self.state.revisions.editor,
			document_text,
			crate::GuiDocumentSyncReason::Bootstrap,
		);
		crate::GuiRuntimeFrame {
			config: self.state.config.clone(),
			revisions: self.state.revisions,
			document_text: document_sync.is_none().then(|| document_text.to_owned()),
			undo_depth,
			redo_depth,
			document_sync,
		}
	}

	pub fn gui_document_fetch(
		&mut self, request: crate::GuiDocumentFetchRequest,
	) -> Result<(crate::GuiDocumentFetchResponse, String), GlorpError> {
		if request.minimum_revision > self.state.revisions.editor {
			return Err(GlorpError::validation(
				None,
				format!(
					"document fetch requested future editor revision `{}` but current revision is `{}`",
					request.minimum_revision, self.state.revisions.editor
				),
			));
		}
		let text = self.state.session.text().to_owned();
		Ok((
			crate::GuiDocumentFetchResponse {
				revision: self.state.revisions.editor,
				bytes: text.len(),
			},
			text,
		))
	}

	pub fn gui_shared_delta(&self, outcome: &glorp_api::GlorpOutcome) -> crate::GuiSharedDelta {
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
