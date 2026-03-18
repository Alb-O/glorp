use {
	glorp_api::{GlorpError, GlorpEvent, GlorpOutcome, GlorpStreamToken, GlorpSubscription},
	std::collections::{BTreeMap, VecDeque},
};

#[derive(Debug, Clone, Default)]
pub struct SubscriptionSet {
	next_token: GlorpStreamToken,
	queues: BTreeMap<GlorpStreamToken, VecDeque<GlorpEvent>>,
}

impl SubscriptionSet {
	pub fn subscribe(&mut self, _request: GlorpSubscription) -> GlorpStreamToken {
		self.next_token += 1;
		self.queues.insert(self.next_token, VecDeque::new());
		self.next_token
	}

	pub fn publish_changed(&mut self, outcome: &GlorpOutcome) {
		let event = GlorpEvent::Changed(outcome.clone());
		for queue in self.queues.values_mut() {
			queue.push_back(event.clone());
		}
	}

	pub fn next_event(&mut self, token: GlorpStreamToken) -> Result<Option<GlorpEvent>, GlorpError> {
		self.queues
			.get_mut(&token)
			.map(VecDeque::pop_front)
			.ok_or_else(|| GlorpError::not_found(format!("unknown subscription token `{token}`")))
	}

	pub fn unsubscribe(&mut self, token: GlorpStreamToken) -> Result<(), GlorpError> {
		self.queues
			.remove(&token)
			.map(|_| ())
			.ok_or_else(|| GlorpError::not_found(format!("unknown subscription token `{token}`")))
	}
}
