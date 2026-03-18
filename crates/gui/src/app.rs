use {
	crate::{GuiMessage, GuiPresentation, GuiTheme, update},
	glorp_api::{GlorpError, GlorpHost, GlorpQuery, GlorpQueryResult, SceneLevel},
};

pub struct GlorpGui<H> {
	host: H,
	theme: GuiTheme,
}

impl<H> GlorpGui<H>
where
	H: GlorpHost,
{
	pub fn new(host: H) -> Self {
		Self {
			host,
			theme: GuiTheme::Classic,
		}
	}

	pub fn theme(&self) -> GuiTheme {
		self.theme
	}

	pub fn send(&mut self, message: GuiMessage) -> Result<(), GlorpError> {
		let command = update::to_command(message)?;
		self.host.execute(command)?;
		Ok(())
	}

	pub fn presentation(&mut self) -> Result<GuiPresentation, GlorpError> {
		match self.host.query(GlorpQuery::Snapshot {
			scene: SceneLevel::IfReady,
			include_document_text: false,
		})? {
			GlorpQueryResult::Snapshot(snapshot) => Ok(GuiPresentation { snapshot }),
			_ => Err(GlorpError::internal("unexpected snapshot response")),
		}
	}

	pub fn into_host(self) -> H {
		self.host
	}
}
