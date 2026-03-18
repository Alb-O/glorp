use {
	glorp_api::{
		GlorpCommand, GlorpError, GlorpEvent, GlorpHost, GlorpOutcome, GlorpQuery, GlorpQueryResult, GlorpStreamToken,
		GlorpSubscription,
	},
	glorp_runtime::RuntimeHost,
	std::sync::{Arc, Mutex},
};

#[derive(Clone)]
pub struct LocalClient {
	host: Arc<Mutex<RuntimeHost>>,
}

impl LocalClient {
	#[must_use]
	pub fn new(host: RuntimeHost) -> Self {
		Self {
			host: Arc::new(Mutex::new(host)),
		}
	}

	#[must_use]
	pub fn shared(host: Arc<Mutex<RuntimeHost>>) -> Self {
		Self { host }
	}

	#[must_use]
	pub fn host(&self) -> Arc<Mutex<RuntimeHost>> {
		Arc::clone(&self.host)
	}

	fn with_host<T>(&self, f: impl FnOnce(&mut RuntimeHost) -> Result<T, GlorpError>) -> Result<T, GlorpError> {
		let mut host = self
			.host
			.lock()
			.map_err(|_| GlorpError::transport("local runtime lock poisoned"))?;
		f(&mut host)
	}
}

impl GlorpHost for LocalClient {
	fn execute(&mut self, command: GlorpCommand) -> Result<GlorpOutcome, GlorpError> {
		self.with_host(|host| host.execute(command))
	}

	fn query(&mut self, query: GlorpQuery) -> Result<GlorpQueryResult, GlorpError> {
		self.with_host(|host| host.query(query))
	}

	fn subscribe(&mut self, request: GlorpSubscription) -> Result<GlorpStreamToken, GlorpError> {
		self.with_host(|host| host.subscribe(request))
	}

	fn next_event(&mut self, token: GlorpStreamToken) -> Result<Option<GlorpEvent>, GlorpError> {
		self.with_host(|host| host.next_event(token))
	}

	fn unsubscribe(&mut self, token: GlorpStreamToken) -> Result<(), GlorpError> {
		self.with_host(|host| host.unsubscribe(token))
	}
}
