use {
	crate::{GlorpError, GlorpRevisions},
	nu_session_core::{CapabilitySet, SessionError, SessionRecord},
	nu_session_protocol_glorp::{
		PROTOCOL_NAME, repo_root as session_repo_root, session_record as glorp_session_record,
	},
};

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct GlorpCapabilities {
	pub transactions: bool,
	pub subscriptions: bool,
	pub streaming: bool,
	pub binary_payloads: bool,
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
		if self.streaming {
			let _ = capabilities.add_feature("streaming")?;
		}
		if self.binary_payloads {
			let _ = capabilities.add_feature("binary-payloads")?;
		}
		Ok(capabilities)
	}

	#[must_use]
	pub fn from_capability_set(capabilities: &CapabilitySet) -> Self {
		Self {
			transactions: capabilities.supports_feature("transactions"),
			subscriptions: capabilities.supports_feature("subscriptions"),
			streaming: capabilities.supports_feature("streaming"),
			binary_payloads: capabilities.supports_feature("binary-payloads"),
			transports: capabilities.transports().into_iter().map(str::to_owned).collect(),
		}
	}
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct DocumentStateView {
	pub revisions: GlorpRevisions,
	pub text_bytes: usize,
	pub text_lines: usize,
	pub undo_depth: usize,
	pub redo_depth: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct GlorpSessionView {
	pub socket: String,
	pub repo_root: Option<String>,
	pub capabilities: GlorpCapabilities,
}

impl GlorpSessionView {
	pub fn session_record(&self) -> Result<SessionRecord, GlorpError> {
		glorp_session_record(
			&self.socket,
			self.repo_root.as_deref(),
			Some(&self.capabilities.capability_set().map_err(session_error)?),
		)
		.map_err(session_error)
	}

	pub fn from_session_record(record: &SessionRecord) -> Result<Self, GlorpError> {
		record.require_transport("ipc").map_err(session_error)?;
		record.require_protocol(PROTOCOL_NAME).map_err(session_error)?;
		Ok(Self {
			socket: record.address.location.clone(),
			repo_root: session_repo_root(record).map(str::to_owned),
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
				streaming: true,
				binary_payloads: true,
				transports: vec!["ipc".to_owned()],
			},
		};

		let record = session.session_record().expect("session record");
		assert_eq!(record.address.transport, "ipc");
		assert_eq!(record.protocol, PROTOCOL_NAME);
		assert!(record.supports_feature("transactions"));
		assert!(record.supports_feature("subscriptions"));
		assert!(record.supports_feature("streaming"));
		assert!(record.supports_feature("binary-payloads"));
		assert_eq!(record.metadata.get("repo_root"), Some(&"/tmp/repo".to_owned()));

		let round_trip = GlorpSessionView::from_session_record(&record).expect("round trip");
		assert_eq!(round_trip, session);
	}
}
