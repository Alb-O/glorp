use {
	glorp_api::*,
	glorp_runtime::RuntimeHost,
	std::sync::{Arc, Mutex},
};

#[derive(Clone)]
pub struct LocalClient {
	host: Arc<Mutex<RuntimeHost>>,
}

impl LocalClient {
	pub fn new(host: RuntimeHost) -> Self {
		Self {
			host: Arc::new(Mutex::new(host)),
		}
	}

	pub fn shared(host: Arc<Mutex<RuntimeHost>>) -> Self {
		Self { host }
	}

	pub fn host(&self) -> Arc<Mutex<RuntimeHost>> {
		Arc::clone(&self.host)
	}
}

impl GlorpHost for LocalClient {
	fn execute(&mut self, command: GlorpCommand) -> Result<GlorpOutcome, GlorpError> {
		self.host
			.lock()
			.map_err(|_| GlorpError::transport("local runtime lock poisoned"))?
			.execute(command)
	}

	fn query(&mut self, query: GlorpQuery) -> Result<GlorpQueryResult, GlorpError> {
		self.host
			.lock()
			.map_err(|_| GlorpError::transport("local runtime lock poisoned"))?
			.query(query)
	}

	fn subscribe(&mut self, request: GlorpSubscription) -> Result<GlorpStreamToken, GlorpError> {
		self.host
			.lock()
			.map_err(|_| GlorpError::transport("local runtime lock poisoned"))?
			.subscribe(request)
	}

	fn next_event(&mut self, token: GlorpStreamToken) -> Result<Option<GlorpEvent>, GlorpError> {
		self.host
			.lock()
			.map_err(|_| GlorpError::transport("local runtime lock poisoned"))?
			.next_event(token)
	}

	fn unsubscribe(&mut self, token: GlorpStreamToken) -> Result<(), GlorpError> {
		self.host
			.lock()
			.map_err(|_| GlorpError::transport("local runtime lock poisoned"))?
			.unsubscribe(token)
	}
}
