use {
	crate::{GuiMessage, GuiPresentation, GuiTheme, update},
	glorp_api::{GlorpError, GlorpHost, GlorpQuery, GlorpQueryResult, SceneLevel, SnapshotQuery},
};

pub struct GlorpGui<H> {
	host: H,
	theme: GuiTheme,
}

impl<H> GlorpGui<H>
where
	H: GlorpHost,
{
	pub const fn new(host: H) -> Self {
		Self {
			host,
			theme: GuiTheme::Classic,
		}
	}

	pub const fn theme(&self) -> GuiTheme {
		self.theme
	}

	pub fn send(&mut self, message: GuiMessage) -> Result<(), GlorpError> {
		self.host.execute(update::to_command(message))?;
		Ok(())
	}

	pub fn presentation(&mut self) -> Result<GuiPresentation, GlorpError> {
		let GlorpQueryResult::Snapshot(snapshot) = self.host.query(GlorpQuery::Snapshot(SnapshotQuery {
			scene: SceneLevel::IfReady,
			include_document_text: false,
		}))?
		else {
			return Err(GlorpError::internal("unexpected snapshot response"));
		};

		Ok(GuiPresentation { snapshot })
	}

	pub fn into_host(self) -> H {
		self.host
	}
}
