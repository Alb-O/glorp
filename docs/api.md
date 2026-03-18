# API

Glorp exposes one semantic contract:

`GlorpCommand` / `GlorpQuery` -> `GlorpOutcome` / `GlorpSnapshot` / `GlorpEvent`

Transactions are typed:

`GlorpTxn { commands: Vec<GlorpCommand> }`

Stable namespaces:

- `config.*`: path-based durable config writes
- `document.*`: whole-document replacement
- `editor.*`: typed motions, mode transitions, edits, history
- `ui.*`: sidebar selection, inspect target selection, viewport scroll, pane ratio
- `scene.*`: explicit scene materialization
- `get selection` / `get inspect-details` / `get perf` / `get ui`: richer read-side projections for live automation
- `session attach` and `events *`: client/session helpers for explicit live-session control over the shared socket

The Nu plugin exposes both executable commands such as `glorp config set`
and typed builder commands such as `glorp cmd config set` for txn assembly.
The CLI is a lower-level JSON bridge around typed `GlorpCommand` / `GlorpQuery` payloads.

See ../schema/glorp-schema.json for the reflection source used by the Nu plugin, generated Nu helpers, and agents.
