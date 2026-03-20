use {
	crate::GuiSharedDelta,
	glorp_api::{GlorpError, GlorpEvent, GlorpOutcome, GlorpStreamToken, GlorpSubscription},
	std::{
		collections::{BTreeMap, VecDeque},
		sync::{Arc, Condvar, Mutex},
		time::Duration,
	},
};

#[derive(Debug, Clone)]
pub struct SubscriptionSet {
	shared: Arc<SharedSubscriptions<GlorpEvent>>,
}

#[derive(Debug, Clone)]
pub struct GuiSubscriptionSet {
	shared: Arc<SharedSubscriptions<GuiSharedDelta>>,
}

#[derive(Debug)]
struct SharedSubscriptions<T> {
	inner: Mutex<SubscriptionState<T>>,
	ready: Condvar,
}

#[derive(Debug, Clone)]
pub struct SubscriptionCheckpoint {
	state: SubscriptionState<GlorpEvent>,
}

#[derive(Debug, Clone)]
struct SubscriptionState<T> {
	next_token: GlorpStreamToken,
	queues: BTreeMap<GlorpStreamToken, VecDeque<T>>,
}

impl<T> Default for SharedSubscriptions<T> {
	fn default() -> Self {
		Self {
			inner: Mutex::new(SubscriptionState::default()),
			ready: Condvar::new(),
		}
	}
}

impl Default for SubscriptionCheckpoint {
	fn default() -> Self {
		Self {
			state: SubscriptionState::default(),
		}
	}
}

impl<T> Default for SubscriptionState<T> {
	fn default() -> Self {
		Self {
			next_token: 0,
			queues: BTreeMap::new(),
		}
	}
}

impl Default for SubscriptionSet {
	fn default() -> Self {
		Self {
			shared: Arc::new(SharedSubscriptions::default()),
		}
	}
}

impl Default for GuiSubscriptionSet {
	fn default() -> Self {
		Self {
			shared: Arc::new(SharedSubscriptions::default()),
		}
	}
}

impl SubscriptionSet {
	pub fn checkpoint(&self) -> SubscriptionCheckpoint {
		SubscriptionCheckpoint {
			state: self
				.shared
				.inner
				.lock()
				.expect("subscription state lock should not be poisoned")
				.clone(),
		}
	}

	pub fn restore(&self, checkpoint: SubscriptionCheckpoint) {
		*self
			.shared
			.inner
			.lock()
			.expect("subscription state lock should not be poisoned") = checkpoint.state;
		self.shared.ready.notify_all();
	}

	pub fn subscribe(&self, _request: GlorpSubscription) -> GlorpStreamToken {
		let mut state = self
			.shared
			.inner
			.lock()
			.expect("subscription state lock should not be poisoned");
		state.next_token += 1;
		let token = state.next_token;
		state.queues.insert(token, VecDeque::new());
		token
	}

	pub fn publish_changed(&self, outcome: &GlorpOutcome) {
		let mut state = self
			.shared
			.inner
			.lock()
			.expect("subscription state lock should not be poisoned");
		if state.queues.is_empty() {
			return;
		}
		let event = GlorpEvent::Changed(outcome.clone());
		for queue in state.queues.values_mut() {
			queue.push_back(event.clone());
		}
		self.shared.ready.notify_all();
	}

	pub fn next_event(&self, token: GlorpStreamToken) -> Result<Option<GlorpEvent>, GlorpError> {
		let mut state = self
			.shared
			.inner
			.lock()
			.expect("subscription state lock should not be poisoned");
		state
			.queues
			.get_mut(&token)
			.map(VecDeque::pop_front)
			.ok_or_else(|| GlorpError::not_found(format!("unknown subscription token `{token}`")))
	}

	pub fn next_event_blocking(
		&self, token: GlorpStreamToken, timeout: Duration,
	) -> Result<Option<GlorpEvent>, GlorpError> {
		let mut state = self
			.shared
			.inner
			.lock()
			.expect("subscription state lock should not be poisoned");
		loop {
			let Some(queue) = state.queues.get_mut(&token) else {
				return Err(GlorpError::not_found(format!("unknown subscription token `{token}`")));
			};
			if let Some(event) = queue.pop_front() {
				return Ok(Some(event));
			}

			let (next_state, wait_result) = self
				.shared
				.ready
				.wait_timeout(state, timeout)
				.expect("subscription wait should not be poisoned");
			state = next_state;
			if wait_result.timed_out() {
				return Ok(None);
			}
		}
	}

	pub fn unsubscribe(&self, token: GlorpStreamToken) -> Result<(), GlorpError> {
		let mut state = self
			.shared
			.inner
			.lock()
			.expect("subscription state lock should not be poisoned");
		state
			.queues
			.remove(&token)
			.map(|_| ())
			.ok_or_else(|| GlorpError::not_found(format!("unknown subscription token `{token}`")))
	}
}

impl GuiSubscriptionSet {
	pub fn subscribe(&self) -> GlorpStreamToken {
		let mut state = self
			.shared
			.inner
			.lock()
			.expect("gui subscription state lock should not be poisoned");
		state.next_token += 1;
		let token = state.next_token;
		state.queues.insert(token, VecDeque::new());
		token
	}

	pub fn publish_changed(&self, delta: &GuiSharedDelta) {
		let mut state = self
			.shared
			.inner
			.lock()
			.expect("gui subscription state lock should not be poisoned");
		if state.queues.is_empty() {
			return;
		}
		for queue in state.queues.values_mut() {
			queue.push_back(delta.clone());
		}
		self.shared.ready.notify_all();
	}

	pub fn next_event_blocking(
		&self, token: GlorpStreamToken, timeout: Duration,
	) -> Result<Option<GuiSharedDelta>, GlorpError> {
		let mut state = self
			.shared
			.inner
			.lock()
			.expect("gui subscription state lock should not be poisoned");
		loop {
			let Some(queue) = state.queues.get_mut(&token) else {
				return Err(GlorpError::not_found(format!("unknown subscription token `{token}`")));
			};
			if let Some(delta) = queue.pop_front() {
				return Ok(Some(delta));
			}

			let (next_state, wait_result) = self
				.shared
				.ready
				.wait_timeout(state, timeout)
				.expect("gui subscription wait should not be poisoned");
			state = next_state;
			if wait_result.timed_out() {
				return Ok(None);
			}
		}
	}

	pub fn unsubscribe(&self, token: GlorpStreamToken) -> Result<(), GlorpError> {
		let mut state = self
			.shared
			.inner
			.lock()
			.expect("gui subscription state lock should not be poisoned");
		state
			.queues
			.remove(&token)
			.map(|_| ())
			.ok_or_else(|| GlorpError::not_found(format!("unknown subscription token `{token}`")))
	}
}
