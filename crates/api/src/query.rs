use {
	crate::{EditorMode, GlorpError, GlorpRevisions, LayoutRectView, WrapChoice},
	nu_session_core::{CapabilitySet, SessionAddress, SessionError, SessionId, SessionRecord},
	nu_session_protocol_glorp::PROTOCOL_NAME,
};

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct GlorpCapabilities {
	pub transactions: bool,
	pub subscriptions: bool,
	pub transports: Vec<String>,
}

impl GlorpCapabilities {
	pub fn capability_set(&self) -> Result<CapabilitySet, SessionError> {
		let mut capabilities = CapabilitySet::new();
		capabilities.extend_transports(self.transports.iter().cloned())?;
		if self.transactions {
			let _ = capabilities.add_feature("transactions")?;
		}
		if self.subscriptions {
			let _ = capabilities.add_feature("subscriptions")?;
		}
		Ok(capabilities)
	}

	#[must_use]
	pub fn from_capability_set(capabilities: &CapabilitySet) -> Self {
		Self {
			transactions: capabilities.supports_feature("transactions"),
			subscriptions: capabilities.supports_feature("subscriptions"),
			transports: capabilities.transports().into_iter().map(str::to_owned).collect(),
		}
	}
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct EditorStateView {
	pub revisions: GlorpRevisions,
	pub mode: EditorMode,
	pub selection: Option<crate::TextRange>,
	pub selected_text: Option<String>,
	pub selection_head: Option<u64>,
	pub pointer_anchor: Option<u64>,
	pub text_bytes: usize,
	pub text_lines: usize,
	pub undo_depth: usize,
	pub redo_depth: usize,
	pub viewport: EditorViewportView,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct EditorViewportView {
	pub wrapping: WrapChoice,
	pub measured_width: f32,
	pub measured_height: f32,
	pub viewport_target: Option<LayoutRectView>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct GlorpSessionView {
	pub socket: String,
	pub repo_root: Option<String>,
	pub capabilities: GlorpCapabilities,
}

impl GlorpSessionView {
	pub fn session_record(&self) -> Result<SessionRecord, GlorpError> {
		let mut record = SessionRecord::new(
			SessionId::new(self.socket.clone()).map_err(session_error)?,
			SessionAddress::new("ipc", self.socket.clone()).map_err(session_error)?,
			PROTOCOL_NAME,
		)
		.map_err(session_error)?;
		record
			.capabilities
			.merge(&self.capabilities.capability_set().map_err(session_error)?);
		if let Some(repo_root) = &self.repo_root {
			let _ = record
				.insert_metadata("repo_root", repo_root.clone())
				.map_err(session_error)?;
		}
		Ok(record)
	}

	pub fn from_session_record(record: &SessionRecord) -> Result<Self, GlorpError> {
		record.require_transport("ipc").map_err(session_error)?;
		record.require_protocol(PROTOCOL_NAME).map_err(session_error)?;
		Ok(Self {
			socket: record.address.location.clone(),
			repo_root: record.metadata.get("repo_root").cloned(),
			capabilities: GlorpCapabilities::from_capability_set(&record.capabilities),
		})
	}
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct GlorpEventStreamView {
	pub token: u64,
	pub subscription: String,
}

fn session_error(error: SessionError) -> GlorpError {
	GlorpError::validation(None, error.to_string())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn session_view_round_trips_through_nu_session_record() {
		let session = GlorpSessionView {
			socket: "/tmp/glorp.sock".to_owned(),
			repo_root: Some("/tmp/repo".to_owned()),
			capabilities: GlorpCapabilities {
				transactions: true,
				subscriptions: true,
				transports: vec!["ipc".to_owned()],
			},
		};

		let record = session.session_record().expect("session record");
		assert_eq!(record.address.transport, "ipc");
		assert_eq!(record.protocol, PROTOCOL_NAME);
		assert!(record.supports_feature("transactions"));
		assert!(record.supports_feature("subscriptions"));
		assert_eq!(record.metadata.get("repo_root"), Some(&"/tmp/repo".to_owned()));

		let round_trip = GlorpSessionView::from_session_record(&record).expect("round trip");
		assert_eq!(round_trip, session);
	}
}
