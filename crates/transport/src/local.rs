use {
	glorp_api::{GlorpCall, GlorpCallResult, GlorpError, GlorpHost},
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
	pub const fn shared(host: Arc<Mutex<RuntimeHost>>) -> Self {
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
	fn call(&mut self, call: GlorpCall) -> Result<GlorpCallResult, GlorpError> {
		self.with_host(|host| host.call(call))
	}
}
