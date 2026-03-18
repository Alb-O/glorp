use {
	crate::{
		GlorpCall, GlorpCallKind, GlorpCallRoute, GlorpCallSpec, GlorpError, GlorpValue, SchemaType, TypeRef,
		catalog::glorp_calls,
	},
	serde::de::DeserializeOwned,
	std::sync::LazyLock,
};

macro_rules! declare_call_specs {
	(
		$(
			$variant:ident {
				id: $id:literal,
				docs: $docs:literal,
				kind: $kind:ident,
				route: $route:ident,
				transactional: $transactional:literal,
				input: $input_kind:ident $(($input:ty))?,
				output: $output:ty,
			}
		),* $(,)?
	) => {
		static CALL_SPECS: LazyLock<Vec<GlorpCallSpec>> = LazyLock::new(|| {
			vec![
				$(
					GlorpCallSpec {
						id: $id.to_owned(),
						kind: GlorpCallKind::$kind,
						route: GlorpCallRoute::$route,
						docs: $docs.to_owned(),
						input: declare_call_input_type!($input_kind $(, $input)?),
						output: named::<$output>(),
						transactional: $transactional,
					},
				)*
			]
		});
	};
}

macro_rules! declare_call_input_type {
	(none) => {
		None
	};
	(some, $input:ty) => {
		Some(named::<$input>())
	};
}

macro_rules! declare_build_call {
	(
		$(
			$variant:ident {
				id: $id:literal,
				docs: $docs:literal,
				kind: $kind:ident,
				route: $route:ident,
				transactional: $transactional:literal,
				input: $input_kind:ident $(($input:ty))?,
				output: $output:ty,
			}
		),* $(,)?
	) => {
		pub fn build_call(id: &str, input: Option<&GlorpValue>) -> Result<GlorpCall, GlorpError> {
			Ok(match id {
				$(
					$id => declare_call_builder!($variant, id, input, $input_kind $(, $input)?),
				)*
				_ => return Err(unknown_call(id)),
			})
		}
	};
}

macro_rules! declare_call_builder {
	($variant:ident, $id:expr, $input:expr, none) => {{
		ensure_no_input($id, $input)?;
		GlorpCall::$variant
	}};
	($variant:ident, $id:expr, $input:expr, some, $ty:ty) => {
		GlorpCall::$variant(decode_required::<$ty>($id, $input)?)
	};
}

glorp_calls!(declare_call_specs);
glorp_calls!(declare_build_call);

pub fn call_specs() -> &'static [GlorpCallSpec] {
	CALL_SPECS.as_slice()
}

pub fn call_spec(id: &str) -> Option<&'static GlorpCallSpec> {
	call_specs().iter().find(|spec| spec.id == id)
}

pub fn call_ids() -> Vec<String> {
	call_specs().iter().map(|call| call.id.clone()).collect()
}

pub fn render_nu_completions() -> String {
	render_completion("call-op", &call_ids())
}

pub fn render_nu_module() -> String {
	"# source this file after registering `nu_plugin_glorp` with `plugin add`\nplugin use glorp\nuse ./completions.nu *\n".to_owned()
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

fn decode_required<T>(id: &str, input: Option<&GlorpValue>) -> Result<T, GlorpError>
where
	T: DeserializeOwned, {
	let Some(input) = input else {
		return Err(GlorpError::validation(None, format!("call `{id}` requires input")));
	};

	serde_json::from_value::<T>(input.into())
		.map_err(|error| GlorpError::validation(None, format!("invalid input for `{id}`: {error}")))
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
