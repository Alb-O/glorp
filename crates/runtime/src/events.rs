use {
	glorp_api::{GlorpError, GlorpEvent, GlorpOutcome, GlorpStreamToken, GlorpSubscription},
	std::{
		collections::{BTreeMap, VecDeque},
		sync::{Arc, Condvar, Mutex},
		time::Duration,
	},
};

#[derive(Debug, Clone)]
pub struct SubscriptionSet {
	shared: Arc<SharedSubscriptions>,
}

#[derive(Debug, Default)]
struct SharedSubscriptions {
	inner: Mutex<SubscriptionState>,
	ready: Condvar,
}

#[derive(Debug, Clone, Default)]
pub struct SubscriptionCheckpoint {
	state: SubscriptionState,
}

#[derive(Debug, Clone, Default)]
struct SubscriptionState {
	next_token: GlorpStreamToken,
	queues: BTreeMap<GlorpStreamToken, VecDeque<GlorpEvent>>,
}

impl Default for SubscriptionSet {
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
