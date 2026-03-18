# Glorp: Nushell-First Runtime Architecture

## Goal

Glorp should expose one deliberate, stable semantic API that becomes the primary scripting, configuration, automation, and agent-control surface. Nushell is the primary interface at that boundary, but it should not directly own or mutate arbitrary internal Rust state.

The product model becomes:

> frontend intent → typed public command/query API → canonical runtime → revisioned events/snapshots

That replaces separate GUI-only semantics, duplicated script vocabularies, and any temptation to expose raw implementation details as public API.

## Current Architectural Seam

The existing codebase already suggests the correct boundary:

- the app shell reduces UI actions into a small effect layer
- effects are executed centrally against the session boundary
- editor/session logic already produces coherent deltas, state, and presentation-friendly snapshots
- there is already a headless/scripted path that hints at a hostable automation surface

That means the public API should sit **above** the current reducer/session split, not below it.

## Core Position

The API should be **fully generated from a deliberately public semantic schema**, not “fully generated from every internal Rust field.”

That distinction matters.

Exposing every internal field would freeze transient and accidental implementation details such as:

- resize coalescers
- focus bookkeeping
- cache internals
- hover plumbing
- perf/debug-only state

Those are not durable product semantics.

Instead, the public surface should cleanly separate:

1. **Durable config**

   - fonts
   - shaping
   - wrapping
   - feature toggles
   - inspect/debug preferences
   - theme and automation settings

2. **Queryable/scriptable state**

   - document text and metadata
   - editor mode
   - selection
   - viewport state
   - scene/materialization state
   - undo/redo availability
   - derived inspect/perf projections

3. **Ephemeral implementation detail**

   - transient widget focus
   - hover caches
   - resize machinery
   - internal scheduling/coalescing mechanics

Category 1 should be stable and writable.
Category 2 should be stable and queryable, with selectively typed commands.
Category 3 should stay private or live behind a debug namespace only.

Public commands should therefore model product semantics, not raw client input.
If a client needs to surface focus, hover, resize, or other widget/plumbing events,
those belong to a private adapter layer or an explicitly unstable debug/client-input namespace,
not the primary contract.

______________________________________________________________________

## Final Workspace Shape

```text
glorp/
  Cargo.poly.toml
  Cargo.catalog.toml

  crates/
    api/
      src/lib.rs
      src/command.rs
      src/query.rs
      src/event.rs
      src/schema.rs
      src/value.rs
      src/config.rs
      src/txn.rs
      src/revision.rs
      src/error.rs

    editor/
      src/lib.rs
      src/document.rs
      src/selection.rs
      src/history.rs
      src/navigation.rs
      src/editing.rs
      src/layout_state.rs
      src/projection.rs
      src/session.rs
      src/types.rs

    runtime/
      src/lib.rs
      src/host.rs
      src/runtime.rs
      src/state.rs
      src/execute.rs
      src/project.rs
      src/events.rs
      src/persistence.rs
      src/config_store.rs
      src/scene.rs
      src/inspect.rs
      src/perf.rs
      src/nu/
        mod.rs
        engine.rs
        config_eval.rs
        script_host.rs
        schema_export.rs

    transport/
      src/lib.rs
      src/local.rs
      src/ipc.rs
      src/client.rs
      src/server.rs

    gui/
      src/lib.rs
      src/launcher.rs
      src/main.rs
      src/app.rs
      src/message.rs
      src/update.rs
      src/view.rs
      src/presenter.rs
      src/sidebar.rs
      src/canvas.rs
      src/theme.rs

    nu-plugin/
      src/lib.rs
      src/main.rs
      src/plugin.rs
      src/commands.rs
      src/completions.rs

    cli/
      src/main.rs
      src/commands.rs
      src/output.rs

  schema/
    glorp-schema.json

  nu/
    glorp.nu
    completions.nu
    default-config.nu

  docs/
    api.md
    config.md
    scripting.md
    transport.md
```

______________________________________________________________________

## Ownership Boundaries

### `glorp_api`

The canonical public contract.

It defines:

- all public commands
- all public queries
- all public events
- typed config
- revisions/deltas/outcomes
- the reflection schema used by Nushell, CLI, and agents

### `glorp_editor`

Pure editing and layout semantics.

It owns:

- document model
- selection and cursor behavior
- editing operations
- navigation/motion
- undo/redo
- layout-facing editor/session state
- scene-facing projections

It does **not** know about:

- GUI widgets
- IPC
- Nushell
- config files
- sidebar tabs
- transport concerns

### `glorp_runtime`

The single canonical state owner.

It owns:

- current typed config
- editor session
- derived scene state
- inspect/perf projections
- persistence
- command execution
- revision tracking
- subscriptions/events
- embedded Nushell evaluation for config and automation

### `glorp_transport`

The stable local protocol between runtime and clients.
It owns the repo-local default socket contract: `glorp.sock`.

### `glorp_gui`

A thin rendering client.

It renders the canonical runtime/editor state, translates widget-local events
into public commands, and exposes the shared repo-local socket contract.

The interactive editor window owns the shared runtime so it can render rich
editor/layout projections directly from the canonical host. The launcher/client
surface may still attach to an already-running socket for non-rendering flows
and acceptance coverage.
It does not define product semantics.

### `glorp_nu_plugin`

Generated from schema.
Not a hand-maintained parallel business-logic layer.

### `glorp_cli`

A non-GUI operator/agent entrypoint using the same contract as all other clients.

______________________________________________________________________

## Canonical Runtime Model

There is one semantic path:

> client intent → `GlorpCommand` / `GlorpQuery` → runtime execute/query → `GlorpOutcome` / `GlorpSnapshot` / `GlorpEvent`

Not:

> GUI message → app action → reducer → session request → separate plugin path

The current reducer/store/session split collapses into:

- public semantic contract in `glorp_api`
- private execution engine in `glorp_runtime`
- thin client adapters in `glorp_gui`, `glorp_cli`, and `glorp_nu_plugin`

The GUI may still have widget-local messages, but those are purely view plumbing.

______________________________________________________________________

## Public Host Contract

```rust
pub trait GlorpHost {
    fn execute(&mut self, command: GlorpCommand) -> Result<GlorpOutcome, GlorpError>;
    fn query(&mut self, query: GlorpQuery) -> Result<GlorpQueryResult, GlorpError>;
    fn subscribe(&mut self, request: GlorpSubscription) -> Result<GlorpStreamToken, GlorpError>;
    fn next_event(&mut self, token: GlorpStreamToken) -> Result<Option<GlorpEvent>, GlorpError>;
    fn unsubscribe(&mut self, token: GlorpStreamToken) -> Result<(), GlorpError>;
}
```

This freezes the system around a small set of concepts:

- execute
- query
- subscribe / read / unsubscribe
- schema

______________________________________________________________________

## Public Command Model

```rust
pub enum GlorpCommand {
    Txn(GlorpTxn),
    Config(ConfigCommand),
    Document(DocumentCommand),
    Editor(EditorCommand),
    Ui(UiCommand),
    Scene(SceneCommand),
}

pub struct GlorpTxn {
    pub commands: Vec<GlorpCommand>,
}
```

### Config commands

```rust
pub enum ConfigCommand {
    Set { path: ConfigPath, value: GlorpValue },
    Patch { values: Vec<ConfigAssignment> },
    Reset { path: ConfigPath },
    Reload,
    Persist,
}
```

`Set`, `Patch`, and `Reset` mutate the effective runtime config only.
`Persist` writes the current effective runtime config to the durable Nu backing file.
`Reload` discards in-memory config mutations and reloads the durable backing file into runtime state.

### Document commands

```rust
pub enum DocumentCommand {
    Replace { text: String },
}
```

### Editor commands

```rust
pub enum EditorCommand {
    Motion(EditorMotion),
    Mode(EditorModeCommand),
    Edit(EditorEditCommand),
    History(EditorHistoryCommand),
    Pointer(EditorPointerCommand),
}
```

### UI commands

```rust
pub enum UiCommand {
    SidebarSelect { tab: SidebarTab },
    InspectTargetSelect { target: Option<CanvasTarget> },
    ViewportScrollTo { x: f32, y: f32 },
    PaneRatioSet { ratio: f32 },
}
```

### Scene commands

```rust
pub enum SceneCommand {
    Ensure,
}
```

### Design rule

Config mutation stays **path-based** for introspection and agent usability.
Editor/UI operations stay **strongly typed enums** rather than collapsing into untyped key/value soup.

That preserves discoverability without sacrificing semantic precision.
Raw client-input events such as hover, focus, and resize remain private adapter concerns unless they are
explicitly surfaced under a non-primary debug/client-input namespace.

______________________________________________________________________

## Query Model

```rust
pub enum GlorpQuery {
    Schema,
    Config,
    Snapshot { scene: SceneLevel, include_document_text: bool },
    DocumentText,
    Capabilities,
}

pub enum GlorpQueryResult {
    Schema(GlorpSchema),
    Config(GlorpConfig),
    Snapshot(GlorpSnapshot),
    DocumentText(String),
    Capabilities(GlorpCapabilities),
}
```

### Scene materialization control

```rust
pub enum SceneLevel {
    Omit,
    IfReady,
    Materialize,
}
```

This gives callers control over whether scene work is omitted, opportunistically included, or forced.

______________________________________________________________________

## Outcome, Delta, and Revisions

```rust
pub enum GlorpEvent {
    Changed(GlorpOutcome),
    Notice(GlorpNotice),
}

pub struct GlorpOutcome {
    pub delta: GlorpDelta,
    pub revisions: GlorpRevisions,
    pub changed_config_paths: Vec<ConfigPath>,
    pub warnings: Vec<GlorpWarning>,
}
```

```rust
pub struct GlorpDelta {
    pub text_changed: bool,
    pub view_changed: bool,
    pub selection_changed: bool,
    pub mode_changed: bool,
    pub config_changed: bool,
    pub ui_changed: bool,
    pub scene_changed: bool,
}

pub struct GlorpRevisions {
    pub editor: u64,
    pub scene: Option<u64>,
    pub config: u64,
}
```

The event model should be revisioned and delta-oriented, not a firehose of full state every time.

______________________________________________________________________

## Values and Path-Based Config

```rust
pub type ConfigPath = String;

pub struct ConfigAssignment {
    pub path: ConfigPath,
    pub value: GlorpValue,
}

pub enum GlorpValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    List(Vec<GlorpValue>),
    Record(BTreeMap<String, GlorpValue>),
}
```

This lets Nushell and coding agents discover, validate, patch, and persist config safely.

______________________________________________________________________

## Public Config Shape

```rust
pub struct GlorpConfig {
    pub editor: EditorConfig,
    pub inspect: InspectConfig,
}

pub struct EditorConfig {
    pub preset: Option<SamplePreset>,
    pub font: FontChoice,
    pub shaping: ShapingChoice,
    pub wrapping: WrapChoice,
    pub font_size: f32,
    pub line_height: f32,
}

pub struct InspectConfig {
    pub show_baselines: bool,
    pub show_hitboxes: bool,
}
```

This can later grow into a broader stable namespace such as:

- `editor.*`
- `viewport.*`
- `inspect.*`
- `theme.*`
- `keys.*`
- `automation.*`

The important part is that config remains **durable and user-facing**, not a dump of runtime internals.

______________________________________________________________________

## Snapshot and Read-Only State Views

```rust
pub struct GlorpSnapshot {
    pub revisions: GlorpRevisions,
    pub config: GlorpConfig,
    pub editor: EditorStateView,
    pub scene: Option<SceneStateView>,
    pub inspect: InspectStateView,
    pub perf: PerfStateView,
    pub ui: UiStateView,
    pub document_text: Option<String>,
}
```

### Editor view

```rust
pub struct EditorStateView {
    pub mode: EditorMode,
    pub selection: Option<TextRange>,
    pub selection_head: Option<u64>,
    pub pointer_anchor: Option<u64>,
    pub text_bytes: usize,
    pub text_lines: usize,
    pub undo_depth: usize,
    pub redo_depth: usize,
    pub viewport: EditorViewportView,
}

pub struct EditorViewportView {
    pub wrapping: WrapChoice,
    pub measured_width: f32,
    pub measured_height: f32,
    pub viewport_target: Option<LayoutRectView>,
}
```

### Scene view

```rust
pub struct SceneStateView {
    pub revision: u64,
    pub measured_width: f32,
    pub measured_height: f32,
    pub run_count: usize,
    pub cluster_count: usize,
}
```

### Inspect/UI view

```rust
pub struct InspectStateView {
    pub hovered_target: Option<CanvasTarget>,
    pub selected_target: Option<CanvasTarget>,
}

pub struct UiStateView {
    pub active_tab: SidebarTab,
    pub canvas_focused: bool,
    pub canvas_scroll_x: f32,
    pub canvas_scroll_y: f32,
    pub layout_width: f32,
    pub viewport_width: f32,
    pub viewport_height: f32,
    pub pane_ratio: f32,
}
```

These are **product views**, not raw runtime structs. They should be intentionally shaped and stable.

______________________________________________________________________

## Schema and Reflection

The schema is central. It is what makes Nushell generation and agent-safe operation actually work.

```rust
pub struct GlorpSchema {
    pub version: u32,
    pub named_types: Vec<NamedTypeSchema>,
    pub config: Vec<ConfigFieldSchema>,
    pub commands: Vec<CommandSchema>,
    pub queries: Vec<QuerySchema>,
    pub events: Vec<EventSchema>,
}
```

```rust
pub struct NamedTypeSchema {
    pub name: String,
    pub docs: String,
    pub kind: TypeSchema,
}

pub struct ConfigFieldSchema {
    pub path: ConfigPath,
    pub docs: String,
    pub ty: TypeRef,
    pub default: GlorpValue,
    pub mutable: bool,
}

pub struct CommandSchema {
    pub path: String,
    pub docs: String,
    pub input: TypeRef,
    pub output: TypeRef,
}

pub struct QuerySchema {
    pub path: String,
    pub docs: String,
    pub output: TypeRef,
}

pub struct EventSchema {
    pub path: String,
    pub docs: String,
    pub payload: TypeRef,
}
```

```rust
pub enum TypeRef {
    Builtin(BuiltinType),
    Named(String),
}

pub enum TypeSchema {
    Enum { variants: Vec<EnumVariantSchema> },
    Record { fields: Vec<FieldSchema> },
    List { item: TypeRef },
    Option { item: TypeRef },
}

pub struct FieldSchema {
    pub name: String,
    pub docs: String,
    pub ty: TypeRef,
    pub required: bool,
}

pub struct EnumVariantSchema {
    pub name: String,
    pub docs: String,
}
```

### Why this matters

Agents need more than setters. They need:

- discoverability
- validation
- enum/value introspection
- docs
- defaults
- typed error surfaces
- proof of what changed after apply

That means schema is not documentation garnish. It is a first-class runtime capability.

______________________________________________________________________

## Generated Nushell Surface

The Nu surface should be generated from schema, but exposed through a hand-designed namespace that stays readable.

### Core commands

```text
glorp schema
glorp get config
glorp get state
glorp get document-text
glorp config set editor.wrapping word
glorp config reset inspect.show-hitboxes
glorp config patch {editor: {font_size: 18}}
glorp editor motion line-end
glorp editor mode enter-insert-after
glorp editor edit insert "hello"
glorp editor history undo
glorp ui sidebar select inspect
glorp ui viewport scroll-to 0 120
glorp scene ensure
glorp txn { ... }
```

The public Nu namespace is intentionally semantic.
It should not grow commands for raw hover/focus/resize input unless those are intentionally exposed as
debug/client-input facilities outside the primary contract.

### Agent-friendly introspection

A coding agent should be able to do things like:

```nu
let field = (glorp schema config | where path == "editor.wrapping" | first)
$field
$field.default
$field.ty
```

Or validate before mutation:

```nu
glorp config validate editor.wrapping "word"
```

The key affordance is **typed discovery before writeback**.

______________________________________________________________________

## Nu as Primary Interface

Nushell is primary at the scripting/config/automation boundary, not as raw owner of private Rust internals.

That means:

- the runtime loads `config.nu`
- config is validated against `GlorpSchema`
- typed `GlorpConfig` is materialized from Nu data
- optional scripted hooks call public semantic commands
- the plugin is a client of the runtime, not a separate logic system

This yields a clean arrangement:

- human scripting
- CLI automation
- agent automation
- GUI control
- headless automation

all use the same command/query/schema vocabulary

______________________________________________________________________

## Durable Config File Shape

`nu/default-config.nu` should be data-first and use canonical kebab-case enum tokens:

```nu
export const config = {
  editor: {
    preset: "tall"
    font: "jetbrains-mono"
    shaping: "advanced"
    wrapping: "word"
    font_size: 24
    line_height: 32
  }

  inspect: {
    show_baselines: false
    show_hitboxes: false
  }
}
```

Optional hooks may live beside this, but they should invoke semantic commands, not reach into runtime internals.

______________________________________________________________________

## Runtime Internals

`glorp_runtime` should split cleanly:

- `state.rs` — canonical runtime state
- `execute.rs` — command execution and transactions
- `project.rs` — snapshot builders and projections
- `events.rs` — subscriptions and revisioned change feed
- `config_store.rs` — load/save/validate Nu config
- `nu/script_host.rs` — in-process Nu execution
- `scene.rs` — scene/materialization ownership
- `inspect.rs` and `perf.rs` — derived projections only

This ensures inspect/perf are not parallel side channels. They become projections off canonical state.

______________________________________________________________________

## Transport

The first-class external shape should be a stable local host protocol.

### Preferred topology

- GUI uses loopback IPC on `glorp.sock`
- Nu plugin uses IPC
- CLI uses IPC and auto-attaches to the repo-local socket when it is live
- agents use CLI or direct IPC

There is no separate “agent API.”
Agents use the same schema, commands, and queries as everything else.

This also makes the headless and automation story honest: one runtime, one protocol, one contract.

______________________________________________________________________

## Mapping from the Existing App

The semantic collapse should look like this:

- current control messages → `ConfigCommand`
- current editor intents → `EditorCommand`
- current sidebar selection, inspect target selection, scroll, and pane layout actions → `UiCommand`
- current focus/hover/resize input stays in private client adapters unless intentionally exposed under a debug/client-input namespace
- current scene materialization → `SceneCommand::Ensure`
- current presentation snapshot → `GlorpSnapshot`
- current reducer/session deltas → `GlorpOutcome`

The current GUI-specific reducer/store/session boundary stops being the product API surface.

______________________________________________________________________

## What Stops Being Architecturally Central

These concepts may still exist privately, but should no longer define the public model:

- app-specific `Message`
- GUI-owned reducer semantics
- separate UI command vocabulary vs scripting vocabulary
- duplicated headless/script harness vocabulary
- direct exposure of internal structs as API

The runtime becomes the semantic owner.
Everything else becomes an adapter.

______________________________________________________________________

## Big-Bang End State

The finished system is:

- one typed semantic API
- one canonical runtime
- one schema used for generation and validation
- one transport surface
- one Nu-generated scripting/config surface
- one thin GUI client
- agents, CLI, and Nu all using the exact same contract

That is the shape that preserves internal flexibility while making Glorp deeply scriptable, introspectable, and agent-accessible.

______________________________________________________________________

## Hard Proof Tests for a Coding Agent

These tests are specifically chosen to make bluffing difficult. They require real end-to-end behavior and stable contracts rather than unit-level handwaving.

### 1. Schema export smoke test

Proves the runtime can actually surface a usable schema.

- start runtime host
- query `Schema`
- assert schema version is nonzero
- assert expected commands exist:
  - `glorp config set`
  - `glorp editor motion`
  - `glorp scene ensure`
- assert config fields include:
  - `editor.font`
  - `editor.wrapping`
  - `inspect.show_hitboxes`
- assert enum docs/defaults are non-empty where required

### 2. Nu plugin round-trip smoke test

Proves the generated Nu surface is actually wired to the runtime.
In practice this is cleanest with Nushell's `nu_plugin_test_support` harness so the test asserts plugin behavior directly without flaky external registration steps.

- boot runtime host
- run:
  ```nu
  glorp get config
  glorp config set editor.wrapping glyph
  glorp get config
  ```
- assert second config read reflects the change
- assert the harness reports no evaluation error
- assert runtime emitted exactly one config revision increment

### 3. Invalid config rejection e2e

Proves validation is real and not just best-effort parsing.

- invoke:
  ```nu
  glorp config set editor.wrapping definitely-not-valid
  ```
- assert nonzero exit status
- assert typed validation error names the path and allowed values
- assert config revision is unchanged
- assert snapshot after failure is byte-for-byte identical to pre-state

### 4. Transaction atomicity e2e

Proves multi-command application is actually transactional.

- start from known config/document state
- execute transaction:
  - valid `config set`
  - valid `editor edit insert`
  - invalid `config set`
- assert transaction fails
- assert document text is unchanged
- assert config is unchanged
- assert no revisions advanced
- assert no `Changed` event was published

### 5. GUI ↔ runtime ↔ snapshot e2e

Proves the GUI is a thin client over the real runtime.

- launch runtime host
- launch GUI connected to it
- drive a GUI interaction that changes sidebar tab and viewport scroll
- query runtime snapshot directly
- assert:
  - `ui.active_tab` changed
  - `ui.canvas_scroll_y` changed
- close GUI
- reconnect another client
- assert runtime state remains coherent

### 6. Editor command to document text e2e

Proves typed editor commands really affect canonical editor state.

- send:
  - `Document::Replace { text: "abc" }`
  - move to line end
  - insert `"!"`
- query `DocumentText`
- assert exact result is `"abc!"`
- assert undo depth increased
- execute undo
- assert exact result returns to `"abc"`

### 7. Scene materialization proof test

Proves scene generation is tied to the runtime, not faked by GUI presentation.

- replace document with fixture text
- query snapshot with `scene = Omit`
- assert `scene == None`
- query snapshot with `scene = Materialize`
- assert `scene != None`
- assert measured width/height and run/cluster counts are nonzero
- assert scene revision appears and is stable across repeated queries with no mutations

### 8. Revision monotonicity test

Makes event sequencing hard to fake.

- capture initial revisions
- apply pure config mutation
- assert only config revision advanced
- apply pure editor mutation
- assert editor revision advanced
- apply scene ensure with no state change
- assert no unrelated revisions advanced
- subscribe to events and verify emitted revisions match query results

### 9. IPC/client parity test

Proves no hidden behavior exists in one entrypoint only.

Run the same sequence through:

- direct in-process host
- IPC client
- Nu plugin
- CLI

For each path:

- set config
- replace document
- insert text
- query snapshot

Assert all final snapshots are semantically identical.

### 10. Persistence smoke test

Proves durable Nu config is real.

- write config through public API
- persist
- stop runtime
- start fresh runtime
- query config
- assert the persisted value loaded
- assert schema validation still passes on load

### 11. Event stream conformance test

Prevents fake “subscription” implementations.

- subscribe
- issue three state-changing commands and one rejected command
- assert exactly three `Changed` events are observed
- assert rejected command emits no `Changed`
- assert each event’s revisions are strictly monotonic
- assert each event’s `changed_config_paths` and delta flags agree with a subsequent queried snapshot

### 12. Golden transcript smoke test

A compact but brutally honest whole-system test.

Run a fixed transcript through CLI or Nu:

```text
glorp config set editor.wrapping word
glorp doc replace "hello"
glorp editor motion line-end
glorp editor edit insert " world"
glorp scene ensure
glorp get state
```

Assert against a golden JSON snapshot containing:

- editor.wrapping = `"word"`
- document text = `"hello world"`
- nonzero editor revision
- present scene revision
- expected undo depth
- stable active tab / viewport defaults

This is simple to run, hard to fake, and ideal for CI.

______________________________________________________________________

## Acceptance Signals

The architecture is in the intended final shape when all of the following are true:

- the GUI can be deleted and reimplemented without changing product semantics
- the Nu plugin is generated from schema and contains no parallel business logic
- CLI, GUI, and agents all use the same transport and public API
- durable config is typed, schema-validated, and round-trippable through Nu
- snapshots/events are revisioned and queryable without GUI participation
- internal runtime fields can change without breaking public consumers unless the semantic contract changes

That is the line between “Nushell-supported” and genuinely “Nushell-first.”
