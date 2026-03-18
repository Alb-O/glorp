use std::collections::BTreeSet;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct GlorpSchema {
	pub version: u32,
	pub types: Vec<NamedTypeSchema>,
	pub calls: Vec<GlorpCallSpec>,
	pub events: Vec<EventSchema>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct NamedTypeSchema {
	pub name: String,
	pub docs: String,
	pub kind: TypeSchema,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct GlorpCallSpec {
	pub id: String,
	pub kind: GlorpCallKind,
	pub route: GlorpCallRoute,
	pub docs: String,
	pub input: Option<TypeRef>,
	pub output: TypeRef,
	pub transactional: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct EventSchema {
	pub id: String,
	pub docs: String,
	pub payload: TypeRef,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum GlorpCallKind {
	Mutation,
	Read,
	Helper,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum GlorpCallRoute {
	Runtime,
	Transport,
	Client,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", content = "value", rename_all = "kebab-case")]
pub enum TypeRef {
	Builtin(BuiltinType),
	Named(String),
	List(Box<TypeRef>),
	Option(Box<TypeRef>),
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum BuiltinType {
	Null,
	Bool,
	Int,
	Float,
	String,
	Any,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum TypeSchema {
	Enum {
		variants: Vec<EnumVariantSchema>,
	},
	TaggedUnion {
		tag: String,
		content: String,
		variants: Vec<TaggedVariantSchema>,
	},
	Record {
		fields: Vec<FieldSchema>,
	},
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct FieldSchema {
	pub name: String,
	pub docs: String,
	pub ty: TypeRef,
	pub required: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct EnumVariantSchema {
	pub name: String,
	pub docs: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct TaggedVariantSchema {
	pub name: String,
	pub docs: String,
	pub payload: Option<TypeRef>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct ConfigFieldSchema {
	pub path: String,
	pub docs: String,
	pub ty: TypeRef,
	pub default: crate::GlorpValue,
	pub mutable: bool,
}

pub trait SchemaType {
	fn type_ref() -> TypeRef;
	fn register(registry: &mut TypeRegistry);
}

#[derive(Debug, Default)]
pub struct TypeRegistry {
	seen: BTreeSet<String>,
	types: Vec<NamedTypeSchema>,
}

impl TypeRegistry {
	pub fn register<T>(&mut self)
	where
		T: SchemaType, {
		T::register(self);
	}

	pub fn into_types(self) -> Vec<NamedTypeSchema> {
		self.types
	}

	fn register_named(&mut self, name: &str, docs: &str, build: impl FnOnce(&mut Self) -> TypeSchema) {
		if !self.seen.insert(name.to_owned()) {
			return;
		}

		let kind = build(self);
		self.types.push(NamedTypeSchema {
			name: name.to_owned(),
			docs: docs.to_owned(),
			kind,
		});
	}
}

impl SchemaType for bool {
	fn type_ref() -> TypeRef {
		TypeRef::Builtin(BuiltinType::Bool)
	}

	fn register(_registry: &mut TypeRegistry) {}
}

impl SchemaType for usize {
	fn type_ref() -> TypeRef {
		TypeRef::Builtin(BuiltinType::Int)
	}

	fn register(_registry: &mut TypeRegistry) {}
}

impl SchemaType for u64 {
	fn type_ref() -> TypeRef {
		TypeRef::Builtin(BuiltinType::Int)
	}

	fn register(_registry: &mut TypeRegistry) {}
}

impl SchemaType for i64 {
	fn type_ref() -> TypeRef {
		TypeRef::Builtin(BuiltinType::Int)
	}

	fn register(_registry: &mut TypeRegistry) {}
}

impl SchemaType for u32 {
	fn type_ref() -> TypeRef {
		TypeRef::Builtin(BuiltinType::Int)
	}

	fn register(_registry: &mut TypeRegistry) {}
}

impl SchemaType for f32 {
	fn type_ref() -> TypeRef {
		TypeRef::Builtin(BuiltinType::Float)
	}

	fn register(_registry: &mut TypeRegistry) {}
}

impl SchemaType for f64 {
	fn type_ref() -> TypeRef {
		TypeRef::Builtin(BuiltinType::Float)
	}

	fn register(_registry: &mut TypeRegistry) {}
}

impl SchemaType for String {
	fn type_ref() -> TypeRef {
		TypeRef::Builtin(BuiltinType::String)
	}

	fn register(_registry: &mut TypeRegistry) {}
}

impl SchemaType for crate::GlorpValue {
	fn type_ref() -> TypeRef {
		TypeRef::Builtin(BuiltinType::Any)
	}

	fn register(_registry: &mut TypeRegistry) {}
}

impl<T> SchemaType for Vec<T>
where
	T: SchemaType,
{
	fn type_ref() -> TypeRef {
		TypeRef::List(Box::new(T::type_ref()))
	}

	fn register(registry: &mut TypeRegistry) {
		T::register(registry);
	}
}

impl<T> SchemaType for Option<T>
where
	T: SchemaType,
{
	fn type_ref() -> TypeRef {
		TypeRef::Option(Box::new(T::type_ref()))
	}

	fn register(registry: &mut TypeRegistry) {
		T::register(registry);
	}
}

macro_rules! impl_named_enum_schema {
	($ty:path, $name:literal, $docs:literal) => {
		impl SchemaType for $ty {
			fn type_ref() -> TypeRef {
				TypeRef::Named($name.to_owned())
			}

			fn register(registry: &mut TypeRegistry) {
				registry.register_named($name, $docs, |_registry| TypeSchema::Enum {
					variants: <$ty as crate::EnumValue>::allowed_values()
						.iter()
						.map(|value| EnumVariantSchema {
							name: (*value).to_owned(),
							docs: <$ty as crate::EnumValue>::docs(value)
								.unwrap_or_default()
								.to_owned(),
						})
						.collect(),
				});
			}
		}
	};
}

macro_rules! impl_named_record_schema {
	($ty:ty, $name:literal, $docs:literal, { $($field:literal => $field_ty:ty : $field_docs:literal),* $(,)? }) => {
		impl SchemaType for $ty {
			fn type_ref() -> TypeRef {
				TypeRef::Named($name.to_owned())
			}

			fn register(registry: &mut TypeRegistry) {
				registry.register_named($name, $docs, |registry| TypeSchema::Record {
					fields: vec![
						$(
							field::<$field_ty>(registry, $field, $field_docs),
						)*
					],
				});
			}
		}
	};
}

fn field<T>(registry: &mut TypeRegistry, name: &str, docs: &str) -> FieldSchema
where
	T: SchemaType, {
	T::register(registry);
	FieldSchema {
		name: name.to_owned(),
		docs: docs.to_owned(),
		ty: T::type_ref(),
		required: true,
	}
}

pub fn glorp_schema() -> GlorpSchema {
	let calls = crate::call_specs().to_vec();
	let events = event_schemas();
	let mut registry = TypeRegistry::default();

	registry.register::<GlorpSchema>();
	for call in &calls {
		if let Some(input) = call.input.as_ref() {
			register_type_ref(&mut registry, input);
		}
		register_type_ref(&mut registry, &call.output);
	}
	for event in &events {
		register_type_ref(&mut registry, &event.payload);
	}

	GlorpSchema {
		version: 6,
		types: registry.into_types(),
		calls,
		events,
	}
}

pub fn event_schemas() -> Vec<EventSchema> {
	vec![
		EventSchema {
			id: "changed".to_owned(),
			docs: "Revisioned state change event.".to_owned(),
			payload: crate::GlorpOutcome::type_ref(),
		},
		EventSchema {
			id: "notice".to_owned(),
			docs: "Runtime notices.".to_owned(),
			payload: crate::GlorpNotice::type_ref(),
		},
	]
}

pub fn named_type(name: &str) -> TypeRef {
	TypeRef::Named(name.to_owned())
}

fn register_type_ref(registry: &mut TypeRegistry, ty: &TypeRef) {
	match ty {
		TypeRef::Builtin(_) => {}
		TypeRef::Named(name) => match name.as_str() {
			"GlorpSchema" => registry.register::<GlorpSchema>(),
			"NamedTypeSchema" => registry.register::<NamedTypeSchema>(),
			"GlorpCallSpec" => registry.register::<GlorpCallSpec>(),
			"EventSchema" => registry.register::<EventSchema>(),
			"TypeRef" => registry.register::<TypeRef>(),
			"TypeSchema" => registry.register::<TypeSchema>(),
			"FieldSchema" => registry.register::<FieldSchema>(),
			"EnumVariantSchema" => registry.register::<EnumVariantSchema>(),
			"TaggedVariantSchema" => registry.register::<TaggedVariantSchema>(),
			"GlorpCallKind" => registry.register::<GlorpCallKind>(),
			"GlorpCallRoute" => registry.register::<GlorpCallRoute>(),
			"BuiltinType" => registry.register::<BuiltinType>(),
			"ConfigAssignment" => registry.register::<crate::ConfigAssignment>(),
			"ConfigPatchInput" => registry.register::<crate::ConfigPatchInput>(),
			"ConfigPathInput" => registry.register::<crate::ConfigPathInput>(),
			"TextInput" => registry.register::<crate::TextInput>(),
			"EditorMotionInput" => registry.register::<crate::EditorMotionInput>(),
			"EditorModeInput" => registry.register::<crate::EditorModeInput>(),
			"EditorHistoryInput" => registry.register::<crate::EditorHistoryInput>(),
			"GlorpTxn" => registry.register::<crate::GlorpTxn>(),
			"GlorpCall" => registry.register::<crate::GlorpCall>(),
			"GlorpCallResult" => registry.register::<crate::GlorpCallResult>(),
			"GlorpEvent" => registry.register::<crate::GlorpEvent>(),
			"GlorpOutcome" => registry.register::<crate::GlorpOutcome>(),
			"GlorpWarning" => registry.register::<crate::GlorpWarning>(),
			"GlorpNotice" => registry.register::<crate::GlorpNotice>(),
			"GlorpRevisions" => registry.register::<crate::GlorpRevisions>(),
			"GlorpDelta" => registry.register::<crate::GlorpDelta>(),
			"GlorpConfig" => registry.register::<crate::GlorpConfig>(),
			"EditorConfig" => registry.register::<crate::EditorConfig>(),
			"GlorpCapabilities" => registry.register::<crate::GlorpCapabilities>(),
			"EditorStateView" => registry.register::<crate::EditorStateView>(),
			"EditorViewportView" => registry.register::<crate::EditorViewportView>(),
			"GlorpSessionView" => registry.register::<crate::GlorpSessionView>(),
			"GlorpEventStreamView" => registry.register::<crate::GlorpEventStreamView>(),
			"OkView" => registry.register::<crate::OkView>(),
			"TokenAckView" => registry.register::<crate::TokenAckView>(),
			"StreamTokenInput" => registry.register::<crate::StreamTokenInput>(),
			"SamplePreset" => registry.register::<crate::SamplePreset>(),
			"WrapChoice" => registry.register::<crate::WrapChoice>(),
			"FontChoice" => registry.register::<crate::FontChoice>(),
			"ShapingChoice" => registry.register::<crate::ShapingChoice>(),
			"EditorMotion" => registry.register::<crate::EditorMotion>(),
			"EditorModeCommand" => registry.register::<crate::EditorModeCommand>(),
			"EditorHistoryCommand" => registry.register::<crate::EditorHistoryCommand>(),
			"EditorMode" => registry.register::<crate::EditorMode>(),
			"TextRange" => registry.register::<crate::TextRange>(),
			"LayoutRectView" => registry.register::<crate::LayoutRectView>(),
			other => panic!("unregistered schema type {other}"),
		},
		TypeRef::List(item) | TypeRef::Option(item) => register_type_ref(registry, item),
	}
}

impl_named_enum_schema!(crate::SamplePreset, "SamplePreset", "Built-in sample document presets.");
impl_named_enum_schema!(crate::WrapChoice, "WrapChoice", "Stable editor wrapping choices.");
impl_named_enum_schema!(crate::FontChoice, "FontChoice", "Stable editor font families.");
impl_named_enum_schema!(crate::ShapingChoice, "ShapingChoice", "Stable shaping choices.");
impl_named_enum_schema!(crate::EditorMotion, "EditorMotion", "Typed editor motions.");
impl_named_enum_schema!(crate::EditorModeCommand, "EditorModeCommand", "Typed mode transitions.");
impl_named_enum_schema!(
	crate::EditorHistoryCommand,
	"EditorHistoryCommand",
	"Typed undo/redo operations."
);
impl_named_enum_schema!(crate::EditorMode, "EditorMode", "Stable editor modes.");

impl SchemaType for GlorpCallKind {
	fn type_ref() -> TypeRef {
		TypeRef::Named("GlorpCallKind".to_owned())
	}

	fn register(registry: &mut TypeRegistry) {
		registry.register_named("GlorpCallKind", "Public call category.", |_registry| TypeSchema::Enum {
			variants: vec![
				EnumVariantSchema {
					name: "mutation".to_owned(),
					docs: "Mutating operation.".to_owned(),
				},
				EnumVariantSchema {
					name: "read".to_owned(),
					docs: "Read-only operation.".to_owned(),
				},
				EnumVariantSchema {
					name: "helper".to_owned(),
					docs: "Helper or control operation.".to_owned(),
				},
			],
		});
	}
}

impl SchemaType for GlorpCallRoute {
	fn type_ref() -> TypeRef {
		TypeRef::Named("GlorpCallRoute".to_owned())
	}

	fn register(registry: &mut TypeRegistry) {
		registry.register_named("GlorpCallRoute", "Where the call is handled.", |_registry| {
			TypeSchema::Enum {
				variants: vec![
					EnumVariantSchema {
						name: "runtime".to_owned(),
						docs: "Handled by the runtime host.".to_owned(),
					},
					EnumVariantSchema {
						name: "transport".to_owned(),
						docs: "Handled by the IPC transport layer.".to_owned(),
					},
					EnumVariantSchema {
						name: "client".to_owned(),
						docs: "Handled locally by the client or plugin.".to_owned(),
					},
				],
			}
		});
	}
}

impl SchemaType for BuiltinType {
	fn type_ref() -> TypeRef {
		TypeRef::Named("BuiltinType".to_owned())
	}

	fn register(registry: &mut TypeRegistry) {
		registry.register_named("BuiltinType", "Builtin scalar categories.", |_registry| {
			TypeSchema::Enum {
				variants: vec![
					EnumVariantSchema {
						name: "null".to_owned(),
						docs: "Null value.".to_owned(),
					},
					EnumVariantSchema {
						name: "bool".to_owned(),
						docs: "Boolean value.".to_owned(),
					},
					EnumVariantSchema {
						name: "int".to_owned(),
						docs: "Integer value.".to_owned(),
					},
					EnumVariantSchema {
						name: "float".to_owned(),
						docs: "Floating-point value.".to_owned(),
					},
					EnumVariantSchema {
						name: "string".to_owned(),
						docs: "String value.".to_owned(),
					},
					EnumVariantSchema {
						name: "any".to_owned(),
						docs: "Arbitrary JSON-like value.".to_owned(),
					},
				],
			}
		});
	}
}

impl_named_record_schema!(NamedTypeSchema, "NamedTypeSchema", "Named schema type.", {
	"name" => String: "Type name.",
	"docs" => String: "Type documentation.",
	"kind" => TypeSchema: "Type shape.",
});
impl_named_record_schema!(GlorpCallSpec, "GlorpCallSpec", "One public call contract.", {
	"id" => String: "Call identifier.",
	"kind" => GlorpCallKind: "Public call category.",
	"route" => GlorpCallRoute: "Where the call is handled.",
	"docs" => String: "Call documentation.",
	"input" => Option<TypeRef>: "Call input type.",
	"output" => TypeRef: "Call output type.",
	"transactional" => bool: "Whether the call is allowed inside `txn`.",
});
impl_named_record_schema!(EventSchema, "EventSchema", "One event contract.", {
	"id" => String: "Event identifier.",
	"docs" => String: "Event documentation.",
	"payload" => TypeRef: "Event payload type.",
});
impl_named_record_schema!(FieldSchema, "FieldSchema", "Record field schema.", {
	"name" => String: "Field name.",
	"docs" => String: "Field documentation.",
	"ty" => TypeRef: "Field value type.",
	"required" => bool: "Whether the field is always present.",
});
impl_named_record_schema!(EnumVariantSchema, "EnumVariantSchema", "String enum variant schema.", {
	"name" => String: "Variant name.",
	"docs" => String: "Variant documentation.",
});
impl_named_record_schema!(TaggedVariantSchema, "TaggedVariantSchema", "Tagged union variant schema.", {
	"name" => String: "Variant name.",
	"docs" => String: "Variant documentation.",
	"payload" => Option<TypeRef>: "Variant payload type.",
});

impl_named_record_schema!(crate::ConfigAssignment, "ConfigAssignment", "One path-based config assignment.", {
	"path" => String: "Config path.",
	"value" => crate::GlorpValue: "Config value.",
});
impl_named_record_schema!(crate::ConfigPatchInput, "ConfigPatchInput", "Nested config patch input.", {
	"patch" => crate::GlorpValue: "Nested config patch record.",
});
impl_named_record_schema!(crate::ConfigPathInput, "ConfigPathInput", "One config path input.", {
	"path" => String: "Config path.",
});
impl_named_record_schema!(crate::TextInput, "TextInput", "One text input.", {
	"text" => String: "Text input.",
});
impl_named_record_schema!(crate::EditorMotionInput, "EditorMotionInput", "One editor motion input.", {
	"motion" => crate::EditorMotion: "Editor motion name.",
});
impl_named_record_schema!(crate::EditorModeInput, "EditorModeInput", "One editor mode input.", {
	"mode" => crate::EditorModeCommand: "Editor mode command.",
});
impl_named_record_schema!(crate::EditorHistoryInput, "EditorHistoryInput", "One editor history input.", {
	"action" => crate::EditorHistoryCommand: "Editor history action.",
});
impl_named_record_schema!(crate::GlorpTxn, "GlorpTxn", "Multiple mutation calls applied atomically.", {
	"calls" => Vec<crate::GlorpCall>: "Ordered mutation calls.",
});
impl_named_record_schema!(crate::StreamTokenInput, "StreamTokenInput", "Subscription token input.", {
	"token" => u64: "Subscription token.",
});
impl_named_record_schema!(crate::GlorpCapabilities, "GlorpCapabilities", "Stable runtime capability flags.", {
	"transactions" => bool: "Whether transactions are supported.",
	"subscriptions" => bool: "Whether subscriptions are supported.",
	"transports" => Vec<String>: "Supported transport names.",
});
impl_named_record_schema!(crate::EditorStateView, "EditorStateView", "Stable editor state view.", {
	"revisions" => crate::GlorpRevisions: "Runtime revisions.",
	"mode" => crate::EditorMode: "Editor mode.",
	"selection" => Option<crate::TextRange>: "Current selection range.",
	"selected_text" => Option<String>: "Selected text if any.",
	"selection_head" => Option<u64>: "Selection head byte offset.",
	"pointer_anchor" => Option<u64>: "Pointer anchor byte offset.",
	"text_bytes" => usize: "Document size in bytes.",
	"text_lines" => usize: "Document line count.",
	"undo_depth" => usize: "Undo depth.",
	"redo_depth" => usize: "Redo depth.",
	"viewport" => crate::EditorViewportView: "Viewport-facing editor metrics.",
});
impl_named_record_schema!(crate::EditorViewportView, "EditorViewportView", "Viewport-facing editor metrics.", {
	"wrapping" => crate::WrapChoice: "Current wrapping mode.",
	"measured_width" => f32: "Measured content width.",
	"measured_height" => f32: "Measured content height.",
	"viewport_target" => Option<crate::LayoutRectView>: "Current viewport reveal target.",
});
impl_named_record_schema!(crate::GlorpSessionView, "GlorpSessionView", "Resolved live session endpoint.", {
	"socket" => String: "Socket path.",
	"repo_root" => Option<String>: "Resolved repo root.",
	"capabilities" => crate::GlorpCapabilities: "Runtime capabilities.",
});
impl_named_record_schema!(crate::GlorpEventStreamView, "GlorpEventStreamView", "Subscription handle for event polling.", {
	"token" => u64: "Subscription token.",
	"subscription" => String: "Subscription name.",
});
impl_named_record_schema!(crate::OkView, "OkView", "Boolean acknowledgement.", {
	"ok" => bool: "Acknowledgement flag.",
});
impl_named_record_schema!(crate::TokenAckView, "TokenAckView", "Acknowledgement with a token payload.", {
	"ok" => bool: "Acknowledgement flag.",
	"token" => u64: "Subscription token.",
});
impl_named_record_schema!(crate::GlorpOutcome, "GlorpOutcome", "Revisioned outcome for a successful command.", {
	"delta" => crate::GlorpDelta: "Change flags.",
	"revisions" => crate::GlorpRevisions: "Post-command revisions.",
	"changed_config_paths" => Vec<String>: "Config paths changed by the command.",
	"warnings" => Vec<crate::GlorpWarning>: "Warnings emitted while handling the operation.",
});
impl_named_record_schema!(crate::GlorpWarning, "GlorpWarning", "Runtime warning payload.", {
	"code" => String: "Warning code.",
	"message" => String: "Warning message.",
});
impl_named_record_schema!(crate::GlorpNotice, "GlorpNotice", "Notice payload.", {
	"code" => String: "Notice code.",
	"message" => String: "Notice message.",
});
impl_named_record_schema!(crate::GlorpRevisions, "GlorpRevisions", "Runtime revisions.", {
	"editor" => u64: "Editor revision.",
	"config" => u64: "Config revision.",
});
impl_named_record_schema!(crate::GlorpDelta, "GlorpDelta", "Boolean change flags.", {
	"text_changed" => bool: "Document text changed.",
	"view_changed" => bool: "Editor view changed.",
	"selection_changed" => bool: "Selection changed.",
	"mode_changed" => bool: "Mode changed.",
	"config_changed" => bool: "Config changed.",
});
impl_named_record_schema!(crate::GlorpConfig, "GlorpConfig", "Stable runtime config.", {
	"editor" => crate::EditorConfig: "Editor config namespace.",
});
impl_named_record_schema!(crate::EditorConfig, "EditorConfig", "Editor configuration namespace.", {
	"preset" => Option<crate::SamplePreset>: "Sample preset.",
	"font" => crate::FontChoice: "Font choice.",
	"shaping" => crate::ShapingChoice: "Shaping choice.",
	"wrapping" => crate::WrapChoice: "Wrapping choice.",
	"font_size" => f32: "Font size.",
	"line_height" => f32: "Line height.",
});
impl_named_record_schema!(crate::TextRange, "TextRange", "Byte range in the document.", {
	"start" => u64: "Start byte offset.",
	"end" => u64: "End byte offset.",
});
impl_named_record_schema!(crate::LayoutRectView, "LayoutRectView", "Rectangle in layout coordinates.", {
	"x" => f32: "Left position.",
	"y" => f32: "Top position.",
	"width" => f32: "Rectangle width.",
	"height" => f32: "Rectangle height.",
});
impl_named_record_schema!(GlorpSchema, "GlorpSchema", "Protocol reflection schema.", {
	"version" => u32: "Schema version.",
	"types" => Vec<NamedTypeSchema>: "Named protocol types.",
	"calls" => Vec<GlorpCallSpec>: "Public call contracts.",
	"events" => Vec<EventSchema>: "Protocol events.",
});

impl SchemaType for TypeRef {
	fn type_ref() -> TypeRef {
		TypeRef::Named("TypeRef".to_owned())
	}

	fn register(registry: &mut TypeRegistry) {
		registry.register_named(
			"TypeRef",
			"Reference to a builtin, named, or composite type.",
			|registry| {
				BuiltinType::register(registry);
				TypeSchema::register(registry);
				TypeSchema::TaggedUnion {
					tag: "kind".to_owned(),
					content: "value".to_owned(),
					variants: vec![
						TaggedVariantSchema {
							name: "builtin".to_owned(),
							docs: "Builtin scalar type.".to_owned(),
							payload: Some(BuiltinType::type_ref()),
						},
						TaggedVariantSchema {
							name: "named".to_owned(),
							docs: "Named schema type.".to_owned(),
							payload: Some(String::type_ref()),
						},
						TaggedVariantSchema {
							name: "list".to_owned(),
							docs: "List of another type.".to_owned(),
							payload: Some(TypeRef::type_ref()),
						},
						TaggedVariantSchema {
							name: "option".to_owned(),
							docs: "Optional version of another type.".to_owned(),
							payload: Some(TypeRef::type_ref()),
						},
					],
				}
			},
		);
	}
}

impl SchemaType for TypeSchema {
	fn type_ref() -> TypeRef {
		TypeRef::Named("TypeSchema".to_owned())
	}

	fn register(registry: &mut TypeRegistry) {
		registry.register_named("TypeSchema", "Shape of a named type.", |registry| {
			FieldSchema::register(registry);
			EnumVariantSchema::register(registry);
			TaggedVariantSchema::register(registry);
			TypeSchema::TaggedUnion {
				tag: "kind".to_owned(),
				content: "value".to_owned(),
				variants: vec![
					TaggedVariantSchema {
						name: "enum".to_owned(),
						docs: "String enum type.".to_owned(),
						payload: Some(Vec::<EnumVariantSchema>::type_ref()),
					},
					TaggedVariantSchema {
						name: "tagged-union".to_owned(),
						docs: "Tagged union type.".to_owned(),
						payload: Some(named_type("TaggedUnionSchema")),
					},
					TaggedVariantSchema {
						name: "record".to_owned(),
						docs: "Record type.".to_owned(),
						payload: Some(Vec::<FieldSchema>::type_ref()),
					},
				],
			}
		});
		registry.register_named("TaggedUnionSchema", "Tagged union schema payload.", |registry| {
			field::<String>(registry, "tag", "Tag field name.");
			field::<String>(registry, "content", "Content field name.");
			field::<Vec<TaggedVariantSchema>>(registry, "variants", "Tagged union variants.");
			TypeSchema::Record {
				fields: vec![
					field::<String>(registry, "tag", "Tag field name."),
					field::<String>(registry, "content", "Content field name."),
					field::<Vec<TaggedVariantSchema>>(registry, "variants", "Tagged union variants."),
				],
			}
		});
	}
}

impl SchemaType for crate::GlorpCall {
	fn type_ref() -> TypeRef {
		TypeRef::Named("GlorpCall".to_owned())
	}

	fn register(registry: &mut TypeRegistry) {
		registry.register_named("GlorpCall", "Raw public call envelope.", |registry| {
			TypeSchema::Record {
				fields: vec![
					field::<String>(registry, "id", "Call identifier."),
					FieldSchema {
						name: "input".to_owned(),
						docs: "Optional raw input payload.".to_owned(),
						ty: Option::<crate::GlorpValue>::type_ref(),
						required: true,
					},
				],
			}
		});
	}
}

impl SchemaType for crate::GlorpCallResult {
	fn type_ref() -> TypeRef {
		TypeRef::Named("GlorpCallResult".to_owned())
	}

	fn register(registry: &mut TypeRegistry) {
		registry.register_named("GlorpCallResult", "Raw public call result envelope.", |registry| {
			TypeSchema::Record {
				fields: vec![
					field::<String>(registry, "id", "Call identifier."),
					field::<crate::GlorpValue>(registry, "output", "Raw output payload."),
				],
			}
		});
	}
}

impl SchemaType for crate::GlorpEvent {
	fn type_ref() -> TypeRef {
		TypeRef::Named("GlorpEvent".to_owned())
	}

	fn register(registry: &mut TypeRegistry) {
		registry.register_named("GlorpEvent", "Runtime event union.", |registry| {
			crate::GlorpOutcome::register(registry);
			crate::GlorpNotice::register(registry);
			TypeSchema::TaggedUnion {
				tag: "kind".to_owned(),
				content: "payload".to_owned(),
				variants: vec![
					TaggedVariantSchema {
						name: "changed".to_owned(),
						docs: "Revisioned state change event.".to_owned(),
						payload: Some(crate::GlorpOutcome::type_ref()),
					},
					TaggedVariantSchema {
						name: "notice".to_owned(),
						docs: "Runtime notice event.".to_owned(),
						payload: Some(crate::GlorpNotice::type_ref()),
					},
				],
			}
		});
	}
}
