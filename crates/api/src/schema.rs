use std::collections::BTreeSet;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct GlorpSchema {
	pub version: u32,
	pub types: Vec<NamedTypeSchema>,
	pub operations: Vec<OperationSpec>,
	pub events: Vec<EventSchema>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct NamedTypeSchema {
	pub name: String,
	pub docs: String,
	pub kind: TypeSchema,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct OperationSpec {
	pub id: String,
	pub kind: OperationKind,
	pub docs: String,
	pub input: Option<TypeRef>,
	pub output: TypeRef,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct EventSchema {
	pub id: String,
	pub docs: String,
	pub payload: TypeRef,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum OperationKind {
	Exec,
	Query,
	Helper,
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
	let operations = crate::operation_specs().to_vec();
	let events = event_schemas();
	let mut registry = TypeRegistry::default();

	registry.register::<GlorpSchema>();
	for operation in &operations {
		if let Some(input) = operation.input.as_ref() {
			register_type_ref(&mut registry, input);
		}
		register_type_ref(&mut registry, &operation.output);
	}
	for event in &events {
		register_type_ref(&mut registry, &event.payload);
	}

	GlorpSchema {
		version: 3,
		types: registry.into_types(),
		operations,
		events,
	}
}

fn operation_variants(registry: &mut TypeRegistry, kind: OperationKind) -> Vec<TaggedVariantSchema> {
	crate::operation_specs()
		.iter()
		.filter(|operation| operation.kind == kind)
		.map(|operation| {
			if let Some(input) = operation.input.as_ref() {
				register_type_ref(registry, input);
			}
			TaggedVariantSchema {
				name: operation.id.clone(),
				docs: operation.docs.clone(),
				payload: operation.input.clone(),
			}
		})
		.collect()
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
			"OperationSpec" => registry.register::<OperationSpec>(),
			"EventSchema" => registry.register::<EventSchema>(),
			"TypeRef" => registry.register::<TypeRef>(),
			"TypeSchema" => registry.register::<TypeSchema>(),
			"FieldSchema" => registry.register::<FieldSchema>(),
			"EnumVariantSchema" => registry.register::<EnumVariantSchema>(),
			"TaggedVariantSchema" => registry.register::<TaggedVariantSchema>(),
			"OperationKind" => registry.register::<OperationKind>(),
			"BuiltinType" => registry.register::<BuiltinType>(),
			"ConfigAssignment" => registry.register::<crate::ConfigAssignment>(),
			"ConfigPatchInput" => registry.register::<crate::ConfigPatchInput>(),
			"ConfigPathInput" => registry.register::<crate::ConfigPathInput>(),
			"TextInput" => registry.register::<crate::TextInput>(),
			"EditorMotionInput" => registry.register::<crate::EditorMotionInput>(),
			"EditorModeInput" => registry.register::<crate::EditorModeInput>(),
			"EditorHistoryInput" => registry.register::<crate::EditorHistoryInput>(),
			"SidebarTabInput" => registry.register::<crate::SidebarTabInput>(),
			"ScrollTarget" => registry.register::<crate::ScrollTarget>(),
			"PaneRatioInput" => registry.register::<crate::PaneRatioInput>(),
			"ViewportMetricsInput" => registry.register::<crate::ViewportMetricsInput>(),
			"CanvasFocusInput" => registry.register::<crate::CanvasFocusInput>(),
			"InspectTargetInput" => registry.register::<crate::InspectTargetInput>(),
			"EditorPointerBeginInput" => registry.register::<crate::EditorPointerBeginInput>(),
			"EditorPointerDragInput" => registry.register::<crate::EditorPointerDragInput>(),
			"GlorpTxn" => registry.register::<crate::GlorpTxn>(),
			"GlorpExec" => registry.register::<crate::GlorpExec>(),
			"GlorpQuery" => registry.register::<crate::GlorpQuery>(),
			"GlorpHelper" => registry.register::<crate::GlorpHelper>(),
			"GlorpEvent" => registry.register::<crate::GlorpEvent>(),
			"GlorpOutcome" => registry.register::<crate::GlorpOutcome>(),
			"GlorpWarning" => registry.register::<crate::GlorpWarning>(),
			"GlorpNotice" => registry.register::<crate::GlorpNotice>(),
			"GlorpRevisions" => registry.register::<crate::GlorpRevisions>(),
			"GlorpDelta" => registry.register::<crate::GlorpDelta>(),
			"GlorpConfig" => registry.register::<crate::GlorpConfig>(),
			"EditorConfig" => registry.register::<crate::EditorConfig>(),
			"InspectConfig" => registry.register::<crate::InspectConfig>(),
			"GlorpCapabilities" => registry.register::<crate::GlorpCapabilities>(),
			"GlorpSnapshot" => registry.register::<crate::GlorpSnapshot>(),
			"EditorStateView" => registry.register::<crate::EditorStateView>(),
			"EditorViewportView" => registry.register::<crate::EditorViewportView>(),
			"SceneStateView" => registry.register::<crate::SceneStateView>(),
			"InspectStateView" => registry.register::<crate::InspectStateView>(),
			"SelectionStateView" => registry.register::<crate::SelectionStateView>(),
			"InspectDetailsView" => registry.register::<crate::InspectDetailsView>(),
			"InspectSceneView" => registry.register::<crate::InspectSceneView>(),
			"PerfStateView" => registry.register::<crate::PerfStateView>(),
			"PerfDashboardView" => registry.register::<crate::PerfDashboardView>(),
			"PerfOverviewView" => registry.register::<crate::PerfOverviewView>(),
			"PerfMetricSummaryView" => registry.register::<crate::PerfMetricSummaryView>(),
			"UiStateView" => registry.register::<crate::UiStateView>(),
			"GlorpSessionView" => registry.register::<crate::GlorpSessionView>(),
			"GlorpEventStreamView" => registry.register::<crate::GlorpEventStreamView>(),
			"OkView" => registry.register::<crate::OkView>(),
			"TokenAckView" => registry.register::<crate::TokenAckView>(),
			"StreamTokenInput" => registry.register::<crate::StreamTokenInput>(),
			"SnapshotQuery" => registry.register::<crate::SnapshotQuery>(),
			"InspectDetailsQuery" => registry.register::<crate::InspectDetailsQuery>(),
			"CanvasTarget" => registry.register::<crate::CanvasTarget>(),
			"SamplePreset" => registry.register::<crate::SamplePreset>(),
			"WrapChoice" => registry.register::<crate::WrapChoice>(),
			"FontChoice" => registry.register::<crate::FontChoice>(),
			"ShapingChoice" => registry.register::<crate::ShapingChoice>(),
			"SidebarTab" => registry.register::<crate::SidebarTab>(),
			"EditorMotion" => registry.register::<crate::EditorMotion>(),
			"EditorModeCommand" => registry.register::<crate::EditorModeCommand>(),
			"EditorHistoryCommand" => registry.register::<crate::EditorHistoryCommand>(),
			"EditorMode" => registry.register::<crate::EditorMode>(),
			"SceneLevel" => registry.register::<crate::SceneLevel>(),
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
impl_named_enum_schema!(crate::SidebarTab, "SidebarTab", "Stable sidebar tabs.");
impl_named_enum_schema!(crate::EditorMotion, "EditorMotion", "Typed editor motions.");
impl_named_enum_schema!(crate::EditorModeCommand, "EditorModeCommand", "Typed mode transitions.");
impl_named_enum_schema!(
	crate::EditorHistoryCommand,
	"EditorHistoryCommand",
	"Typed undo/redo operations."
);
impl_named_enum_schema!(crate::EditorMode, "EditorMode", "Stable editor modes.");

impl SchemaType for OperationKind {
	fn type_ref() -> TypeRef {
		TypeRef::Named("OperationKind".to_owned())
	}

	fn register(registry: &mut TypeRegistry) {
		registry.register_named("OperationKind", "Operation category.", |_registry| TypeSchema::Enum {
			variants: vec![
				EnumVariantSchema {
					name: "exec".to_owned(),
					docs: "Mutating operation.".to_owned(),
				},
				EnumVariantSchema {
					name: "query".to_owned(),
					docs: "Read-only operation.".to_owned(),
				},
				EnumVariantSchema {
					name: "helper".to_owned(),
					docs: "Plugin-side helper operation.".to_owned(),
				},
			],
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
impl_named_record_schema!(OperationSpec, "OperationSpec", "One protocol operation.", {
	"id" => String: "Operation identifier.",
	"kind" => OperationKind: "Operation category.",
	"docs" => String: "Operation documentation.",
	"input" => Option<TypeRef>: "Operation input type.",
	"output" => TypeRef: "Operation output type.",
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
impl_named_record_schema!(crate::SidebarTabInput, "SidebarTabInput", "One sidebar tab input.", {
	"tab" => crate::SidebarTab: "Sidebar tab.",
});
impl_named_record_schema!(crate::ScrollTarget, "ScrollTarget", "Viewport scroll target.", {
	"x" => f32: "Horizontal scroll offset.",
	"y" => f32: "Vertical scroll offset.",
});
impl_named_record_schema!(crate::PaneRatioInput, "PaneRatioInput", "Sidebar/canvas ratio input.", {
	"ratio" => f32: "Sidebar/canvas ratio.",
});
impl_named_record_schema!(crate::ViewportMetricsInput, "ViewportMetricsInput", "Viewport metrics update.", {
	"layout_width" => f32: "Measured layout width.",
	"viewport_width" => f32: "Measured viewport width.",
	"viewport_height" => f32: "Measured viewport height.",
});
impl_named_record_schema!(crate::CanvasFocusInput, "CanvasFocusInput", "Canvas focus change input.", {
	"focused" => bool: "Whether the canvas owns focus.",
});
impl_named_record_schema!(crate::InspectTargetInput, "InspectTargetInput", "Inspect target input.", {
	"target" => Option<crate::CanvasTarget>: "Optional inspect target.",
});
impl_named_record_schema!(crate::EditorPointerBeginInput, "EditorPointerBeginInput", "Pointer press input.", {
	"x" => f32: "Pointer x position.",
	"y" => f32: "Pointer y position.",
	"select_word" => bool: "Whether to select a word.",
});
impl_named_record_schema!(crate::EditorPointerDragInput, "EditorPointerDragInput", "Pointer drag input.", {
	"x" => f32: "Pointer x position.",
	"y" => f32: "Pointer y position.",
});
impl_named_record_schema!(crate::GlorpTxn, "GlorpTxn", "Multiple exec operations applied atomically.", {
	"execs" => Vec<crate::GlorpExec>: "Ordered exec operations.",
});
impl_named_record_schema!(crate::StreamTokenInput, "StreamTokenInput", "Subscription token input.", {
	"token" => u64: "Subscription token.",
});
impl_named_record_schema!(crate::SnapshotQuery, "SnapshotQuery", "Snapshot query input.", {
	"scene" => crate::SceneLevel: "Scene materialization policy.",
	"include_document_text" => bool: "Whether to include document text.",
});
impl_named_record_schema!(crate::InspectDetailsQuery, "InspectDetailsQuery", "Inspect-details query input.", {
	"target" => Option<crate::CanvasTarget>: "Optional active inspect target.",
});
impl_named_record_schema!(crate::GlorpCapabilities, "GlorpCapabilities", "Stable runtime capability flags.", {
	"transactions" => bool: "Whether transactions are supported.",
	"subscriptions" => bool: "Whether subscriptions are supported.",
	"transports" => Vec<String>: "Supported transport names.",
});
impl_named_record_schema!(crate::GlorpSnapshot, "GlorpSnapshot", "Read-only runtime snapshot.", {
	"revisions" => crate::GlorpRevisions: "Runtime revisions.",
	"config" => crate::GlorpConfig: "Effective config.",
	"editor" => crate::EditorStateView: "Editor state view.",
	"scene" => Option<crate::SceneStateView>: "Scene state view.",
	"inspect" => crate::InspectStateView: "Inspect state view.",
	"perf" => crate::PerfStateView: "Performance state view.",
	"ui" => crate::UiStateView: "UI state view.",
	"document_text" => Option<String>: "Document text when requested.",
});
impl_named_record_schema!(crate::EditorStateView, "EditorStateView", "Stable editor state view.", {
	"mode" => crate::EditorMode: "Editor mode.",
	"selection" => Option<crate::TextRange>: "Current selection range.",
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
impl_named_record_schema!(crate::SceneStateView, "SceneStateView", "Stable scene state view.", {
	"revision" => u64: "Scene revision.",
	"measured_width" => f32: "Measured width.",
	"measured_height" => f32: "Measured height.",
	"run_count" => usize: "Layout run count.",
	"cluster_count" => usize: "Layout cluster count.",
});
impl_named_record_schema!(crate::InspectStateView, "InspectStateView", "Stable inspect state view.", {
	"hovered_target" => Option<crate::CanvasTarget>: "Hovered inspect target.",
	"selected_target" => Option<crate::CanvasTarget>: "Selected inspect target.",
});
impl_named_record_schema!(crate::SelectionStateView, "SelectionStateView", "Focused selection read model.", {
	"mode" => crate::EditorMode: "Editor mode.",
	"range" => Option<crate::TextRange>: "Current selection range.",
	"selected_text" => Option<String>: "Selected text if any.",
	"selection_head" => Option<u64>: "Selection head byte offset.",
	"pointer_anchor" => Option<u64>: "Pointer anchor byte offset.",
	"viewport_target" => Option<crate::LayoutRectView>: "Current viewport reveal target.",
});
impl_named_record_schema!(crate::InspectDetailsView, "InspectDetailsView", "Rich inspect read model.", {
	"hovered_target" => Option<crate::CanvasTarget>: "Hovered inspect target.",
	"selected_target" => Option<crate::CanvasTarget>: "Selected inspect target.",
	"active_target" => Option<crate::CanvasTarget>: "Active inspect target.",
	"warnings" => Vec<String>: "Scene warnings.",
	"interaction_details" => String: "Human-readable target details.",
	"scene" => Option<crate::InspectSceneView>: "Inspect scene summary.",
});
impl_named_record_schema!(crate::InspectSceneView, "InspectSceneView", "Inspect-side scene summary.", {
	"revision" => u64: "Scene revision.",
	"run_count" => usize: "Layout run count.",
	"cluster_count" => usize: "Layout cluster count.",
});
impl_named_record_schema!(crate::PerfStateView, "PerfStateView", "Stable runtime perf counters.", {
	"scene_builds" => usize: "Scene build count.",
	"scene_build_millis" => f64: "Accumulated scene build millis.",
});
impl_named_record_schema!(crate::PerfDashboardView, "PerfDashboardView", "Rich runtime perf dashboard.", {
	"overview" => crate::PerfOverviewView: "Perf overview.",
	"metrics" => Vec<crate::PerfMetricSummaryView>: "Perf metric summaries.",
});
impl_named_record_schema!(crate::PerfOverviewView, "PerfOverviewView", "Perf overview summary.", {
	"editor_mode" => crate::EditorMode: "Editor mode.",
	"editor_bytes" => usize: "Document size in bytes.",
	"text_lines" => usize: "Document line count.",
	"layout_width" => f32: "Current layout width.",
	"scene_ready" => bool: "Whether scene data is materialized.",
	"scene_revision" => Option<u64>: "Current scene revision.",
	"scene_width" => f32: "Scene width.",
	"scene_height" => f32: "Scene height.",
	"run_count" => usize: "Layout run count.",
	"cluster_count" => usize: "Layout cluster count.",
	"warning_count" => usize: "Scene warning count.",
});
impl_named_record_schema!(crate::PerfMetricSummaryView, "PerfMetricSummaryView", "Perf metric summary row.", {
	"label" => String: "Metric label.",
	"total_samples" => u64: "Total samples.",
	"total_millis" => f64: "Total millis.",
	"last_millis" => f64: "Most recent millis.",
	"avg_millis" => f64: "Average millis.",
});
impl_named_record_schema!(crate::UiStateView, "UiStateView", "Stable UI state view.", {
	"active_tab" => crate::SidebarTab: "Active sidebar tab.",
	"canvas_focused" => bool: "Whether the canvas owns focus.",
	"canvas_scroll_x" => f32: "Horizontal scroll offset.",
	"canvas_scroll_y" => f32: "Vertical scroll offset.",
	"layout_width" => f32: "Current layout width.",
	"viewport_width" => f32: "Viewport width.",
	"viewport_height" => f32: "Viewport height.",
	"pane_ratio" => f32: "Sidebar/canvas ratio.",
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
	"scene" => Option<u64>: "Scene revision if materialized.",
	"config" => u64: "Config revision.",
});
impl_named_record_schema!(crate::GlorpDelta, "GlorpDelta", "Boolean change flags.", {
	"text_changed" => bool: "Document text changed.",
	"view_changed" => bool: "Editor view changed.",
	"selection_changed" => bool: "Selection changed.",
	"mode_changed" => bool: "Mode changed.",
	"config_changed" => bool: "Config changed.",
	"ui_changed" => bool: "UI state changed.",
	"scene_changed" => bool: "Scene state changed.",
});
impl_named_record_schema!(crate::GlorpConfig, "GlorpConfig", "Stable runtime config.", {
	"editor" => crate::EditorConfig: "Editor config namespace.",
	"inspect" => crate::InspectConfig: "Inspect config namespace.",
});
impl_named_record_schema!(crate::EditorConfig, "EditorConfig", "Editor configuration namespace.", {
	"preset" => Option<crate::SamplePreset>: "Sample preset.",
	"font" => crate::FontChoice: "Font choice.",
	"shaping" => crate::ShapingChoice: "Shaping choice.",
	"wrapping" => crate::WrapChoice: "Wrapping choice.",
	"font_size" => f32: "Font size.",
	"line_height" => f32: "Line height.",
});
impl_named_record_schema!(crate::InspectConfig, "InspectConfig", "Inspect configuration namespace.", {
	"show_baselines" => bool: "Show baselines.",
	"show_hitboxes" => bool: "Show hitboxes.",
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
	"operations" => Vec<OperationSpec>: "Protocol operations.",
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

impl SchemaType for crate::CanvasTarget {
	fn type_ref() -> TypeRef {
		TypeRef::Named("CanvasTarget".to_owned())
	}

	fn register(registry: &mut TypeRegistry) {
		registry.register_named("CanvasTarget", "Canvas inspect target.", |_registry| {
			TypeSchema::TaggedUnion {
				tag: "kind".to_owned(),
				content: "value".to_owned(),
				variants: vec![
					TaggedVariantSchema {
						name: "run".to_owned(),
						docs: "Run index target.".to_owned(),
						payload: Some(usize::type_ref()),
					},
					TaggedVariantSchema {
						name: "cluster".to_owned(),
						docs: "Cluster index target.".to_owned(),
						payload: Some(usize::type_ref()),
					},
				],
			}
		});
	}
}

impl SchemaType for crate::SceneLevel {
	fn type_ref() -> TypeRef {
		TypeRef::Named("SceneLevel".to_owned())
	}

	fn register(registry: &mut TypeRegistry) {
		registry.register_named("SceneLevel", "Scene materialization policies.", |_registry| {
			TypeSchema::Enum {
				variants: vec![
					EnumVariantSchema {
						name: "omit".to_owned(),
						docs: "Do not include scene state.".to_owned(),
					},
					EnumVariantSchema {
						name: "if-ready".to_owned(),
						docs: "Include scene state when already materialized.".to_owned(),
					},
					EnumVariantSchema {
						name: "materialize".to_owned(),
						docs: "Materialize scene state before returning.".to_owned(),
					},
				],
			}
		});
	}
}

impl SchemaType for crate::GlorpExec {
	fn type_ref() -> TypeRef {
		TypeRef::Named("GlorpExec".to_owned())
	}

	fn register(registry: &mut TypeRegistry) {
		registry.register_named("GlorpExec", "Typed exec operations.", |registry| {
			TypeSchema::TaggedUnion {
				tag: "op".to_owned(),
				content: "input".to_owned(),
				variants: operation_variants(registry, OperationKind::Exec),
			}
		});
	}
}

impl SchemaType for crate::GlorpQuery {
	fn type_ref() -> TypeRef {
		TypeRef::Named("GlorpQuery".to_owned())
	}

	fn register(registry: &mut TypeRegistry) {
		registry.register_named("GlorpQuery", "Typed query operations.", |registry| {
			TypeSchema::TaggedUnion {
				tag: "op".to_owned(),
				content: "input".to_owned(),
				variants: operation_variants(registry, OperationKind::Query),
			}
		});
	}
}

impl SchemaType for crate::GlorpHelper {
	fn type_ref() -> TypeRef {
		TypeRef::Named("GlorpHelper".to_owned())
	}

	fn register(registry: &mut TypeRegistry) {
		registry.register_named("GlorpHelper", "Typed helper operations.", |registry| {
			TypeSchema::TaggedUnion {
				tag: "op".to_owned(),
				content: "input".to_owned(),
				variants: operation_variants(registry, OperationKind::Helper),
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
