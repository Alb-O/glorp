use {
	crate::{
		GlorpCall, GlorpCallKind, GlorpCallResult, GlorpCallRoute, GlorpCallSpec, GlorpCaller, GlorpError,
		GlorpOutcome, GlorpValue, SchemaType, TypeRef,
	},
	serde::{Serialize, de::DeserializeOwned},
	std::collections::BTreeSet,
};

pub trait GlorpCallOutput: Sized {
	fn encode(self, id: &str) -> Result<GlorpValue, GlorpError>;
	fn decode(id: &str, output: &GlorpValue) -> Result<Self, GlorpError>;
}

impl<T> GlorpCallOutput for T
where
	T: Serialize + DeserializeOwned,
{
	fn encode(self, id: &str) -> Result<GlorpValue, GlorpError> {
		encode_value(id, "output", self)
	}

	fn decode(id: &str, output: &GlorpValue) -> Result<Self, GlorpError> {
		decode_value(id, "output", output)
	}
}

pub trait GlorpCallDescriptor {
	type Input;
	type Output: GlorpCallOutput;

	const ID: &'static str;
	const DOCS: &'static str;
	const KIND: GlorpCallKind;
	const ROUTE: GlorpCallRoute;
	const TRANSACTIONAL: bool;

	fn input_type() -> Option<TypeRef>;
	fn output_type() -> TypeRef;

	fn build(input: Self::Input) -> Result<GlorpCall, GlorpError>;
	fn build_raw(input: Option<&GlorpValue>) -> Result<GlorpCall, GlorpError>;
	fn decode_call_input(call: &GlorpCall) -> Result<Self::Input, GlorpError>;

	fn respond(output: Self::Output) -> Result<GlorpCallResult, GlorpError> {
		Ok(GlorpCallResult {
			id: Self::ID.to_owned(),
			output: output.encode(Self::ID)?,
		})
	}

	fn decode_result_output(result: GlorpCallResult) -> Result<Self::Output, GlorpError> {
		ensure_call_id(Self::ID, &result.id)?;
		Self::Output::decode(Self::ID, &result.output)
	}

	fn decode_output(output: &GlorpValue) -> Result<Self::Output, GlorpError> {
		Self::Output::decode(Self::ID, output)
	}

	fn call(caller: &mut (impl GlorpCaller + ?Sized), input: Self::Input) -> Result<Self::Output, GlorpError> {
		Self::decode_result_output(caller.call(Self::build(input)?)?)
	}
}

pub trait GlorpCallerExt: GlorpCaller {
	fn call_typed<D>(&mut self, input: D::Input) -> Result<D::Output, GlorpError>
	where
		D: GlorpCallDescriptor, {
		D::call(self, input)
	}
}

impl<T> GlorpCallerExt for T where T: GlorpCaller + ?Sized {}

#[path = "generated_calls.rs"]
mod generated_calls;

pub use self::generated_calls::*;

pub fn render_nu_completions() -> String {
	render_completion("call-op", &call_ids())
}

pub fn render_nu_module() -> String {
	"# source this file after registering `nu_plugin_glorp` with `plugin add`\nplugin use glorp\nuse ./completions.nu *\n".to_owned()
}

pub fn catalog_invariants() -> Result<(), String> {
	validate_catalog_invariants(call_specs())
}

pub fn route_dispatch_error(id: &str, _expected_route: GlorpCallRoute) -> GlorpError {
	match call_spec(id) {
		Some(spec) => GlorpError::validation(
			None,
			format!("call `{id}` must be handled by the {:?} route", spec.route).to_lowercase(),
		),
		None => unknown_call(id),
	}
}

pub fn transactional_call_spec(call: &GlorpCall) -> Result<&'static GlorpCallSpec, GlorpError> {
	if call.id == calls::Txn::ID {
		return Err(GlorpError::validation(None, "nested transactions are not supported"));
	}

	let Some(spec) = call_spec(&call.id) else {
		return Err(unknown_call(&call.id));
	};

	if spec.route != GlorpCallRoute::Runtime || !spec.transactional {
		return Err(GlorpError::validation(
			None,
			format!("call `{}` is not allowed inside `txn`", spec.id),
		));
	}

	Ok(spec)
}

pub fn decode_call_output<T>(id: &str, output: &GlorpValue) -> Result<T, GlorpError>
where
	T: GlorpCallOutput, {
	T::decode(id, output)
}

fn named<T>() -> TypeRef
where
	T: SchemaType, {
	T::type_ref()
}

fn ensure_no_input(id: &str, input: Option<&GlorpValue>) -> Result<(), GlorpError> {
	match input {
		None | Some(GlorpValue::Null) => Ok(()),
		Some(_) => Err(GlorpError::validation(
			None,
			format!("call `{id}` does not accept input"),
		)),
	}
}

fn decode_required<T>(id: &str, field: &str, value: Option<&GlorpValue>) -> Result<T, GlorpError>
where
	T: DeserializeOwned, {
	let Some(value) = value else {
		return Err(GlorpError::validation(None, format!("call `{id}` requires {field}")));
	};

	decode_value(id, field, value)
}

fn decode_value<T>(id: &str, field: &str, value: &GlorpValue) -> Result<T, GlorpError>
where
	T: DeserializeOwned, {
	serde_json::from_value::<T>(value.into())
		.map_err(|error| GlorpError::validation(None, format!("invalid {field} for `{id}`: {error}")))
}

fn encode_value<T>(id: &str, field: &str, value: T) -> Result<GlorpValue, GlorpError>
where
	T: Serialize, {
	serde_json::to_value(value)
		.map(GlorpValue::from)
		.map_err(|error| GlorpError::internal(format!("failed to encode {field} for `{id}`: {error}")))
}

fn ensure_call_id(expected: &str, actual: &str) -> Result<(), GlorpError> {
	if actual == expected {
		Ok(())
	} else {
		Err(GlorpError::validation(
			None,
			format!("expected `{expected}` envelope, got `{actual}`"),
		))
	}
}

fn unknown_call(id: &str) -> GlorpError {
	GlorpError::not_found(format!("unknown call `{id}`"))
}

fn render_completion(name: &str, values: &[String]) -> String {
	let values = values
		.iter()
		.map(|value| format!("\"{value}\""))
		.collect::<Vec<_>>()
		.join(" ");
	format!("export def \"nu-complete glorp {name}\" [] {{ [{values}] }}\n")
}

fn debug_assert_catalog_invariants(specs: &[GlorpCallSpec]) {
	if cfg!(debug_assertions) {
		assert!(validate_catalog_invariants(specs).is_ok());
	}
}

fn validate_catalog_invariants(specs: &[GlorpCallSpec]) -> Result<(), String> {
	let mut ids = BTreeSet::new();
	for spec in specs {
		if !ids.insert(spec.id.clone()) {
			return Err(format!("duplicate call id `{}`", spec.id));
		}
		if spec.transactional && spec.route != GlorpCallRoute::Runtime {
			return Err(format!("transactional call `{}` must use the runtime route", spec.id));
		}
		if spec.transactional && spec.output != named::<GlorpOutcome>() {
			return Err(format!("transactional call `{}` must return `GlorpOutcome`", spec.id));
		}
	}

	let Some(txn) = specs.iter().find(|spec| spec.id == calls::Txn::ID) else {
		return Err("missing `txn` call".to_owned());
	};
	if txn.transactional {
		return Err("`txn` must not be transactional".to_owned());
	}

	Ok(())
}

#[cfg(test)]
mod tests {
	use {
		super::*,
		crate::{ConfigAssignment, GlorpEvent, GlorpNotice, StreamTokenInput, TextInput},
	};

	#[test]
	fn raw_call_roundtrip() {
		let call = calls::DocumentReplace::build(TextInput {
			text: "hello".to_owned(),
		})
		.expect("call should build");
		let encoded = serde_json::to_string(&call).expect("call should serialize");
		let decoded: GlorpCall = serde_json::from_str(&encoded).expect("call should deserialize");
		assert_eq!(decoded, call);
	}

	#[test]
	fn raw_call_result_roundtrip() {
		let result = calls::EventsNext::respond(Some(GlorpEvent::Notice(GlorpNotice {
			code: "demo".to_owned(),
			message: "demo".to_owned(),
		})))
		.expect("result should build");
		let encoded = serde_json::to_string(&result).expect("result should serialize");
		let decoded: GlorpCallResult = serde_json::from_str(&encoded).expect("result should deserialize");
		assert_eq!(decoded, result);
	}

	#[test]
	fn catalog_invariants_hold() {
		assert_eq!(catalog_invariants(), Ok(()));
	}

	#[test]
	fn typed_helper_handles_no_input() {
		let call = calls::Capabilities::build(()).expect("call should build");
		assert_eq!(call.id, "capabilities");
		assert_eq!(call.input, None);
	}

	#[test]
	fn typed_helper_handles_required_input() {
		let input = ConfigAssignment {
			path: "editor.wrapping".to_owned(),
			value: GlorpValue::String("word".to_owned()),
		};
		let call = calls::ConfigSet::build(input.clone()).expect("call should build");
		let decoded = calls::ConfigSet::decode_call_input(&call).expect("input should decode");
		assert_eq!(decoded, input);
	}

	#[test]
	fn typed_helper_handles_optional_output() {
		let result = calls::EventsNext::respond(None).expect("result should build");
		let decoded = calls::EventsNext::decode_result_output(result).expect("output should decode");
		assert_eq!(decoded, None);
	}

	#[test]
	fn build_call_rejects_wrong_shape() {
		let error = build_call("document", Some(&GlorpValue::Bool(true))).expect_err("call should fail");
		assert!(matches!(error, GlorpError::Validation { .. }));
	}

	#[test]
	fn build_call_result_rejects_wrong_shape() {
		let error =
			build_call_result("config", &GlorpValue::String("oops".to_owned())).expect_err("result should fail");
		assert!(matches!(error, GlorpError::Validation { .. }));
	}

	#[test]
	fn transactional_call_validation_uses_raw_id() {
		let call = GlorpCall {
			id: calls::SessionShutdown::ID.to_owned(),
			input: None,
		};
		let error = transactional_call_spec(&call).expect_err("transport call should fail");
		assert!(matches!(error, GlorpError::Validation { .. }));
	}

	#[test]
	fn descriptor_metadata_matches_spec_table() {
		let spec = call_spec(calls::DocumentReplace::ID).expect("spec should exist");
		assert_eq!(spec.route, calls::DocumentReplace::ROUTE);
		assert_eq!(spec.input, Some(named::<TextInput>()));
		assert_eq!(spec.output, named::<GlorpOutcome>());
	}

	#[test]
	fn decode_call_output_supports_optional_payloads() {
		let output = calls::EventsNext::respond(None).expect("result should build").output;
		let decoded: Option<GlorpEvent> =
			decode_call_output(calls::EventsNext::ID, &output).expect("output should decode");
		assert_eq!(decoded, None);
	}

	#[test]
	fn build_call_supports_nested_raw_transactions() {
		let nested = calls::DocumentReplace::build(TextInput {
			text: "nested".to_owned(),
		})
		.expect("nested call should build");
		let txn = calls::Txn::build(crate::GlorpTxn { calls: vec![nested] }).expect("txn should build");
		let decoded = calls::Txn::decode_call_input(&txn).expect("txn should decode");
		assert_eq!(decoded.calls.len(), 1);
	}

	#[test]
	fn build_call_from_raw_value_supports_required_input() {
		let raw = GlorpValue::Record([("token".to_owned(), GlorpValue::Int(41))].into_iter().collect());
		let call = build_call(calls::EventsNext::ID, Some(&raw)).expect("call should build");
		let decoded = calls::EventsNext::decode_call_input(&call).expect("input should decode");
		assert_eq!(decoded, StreamTokenInput { token: 41 });
	}
}
