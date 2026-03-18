# API

Glorp exposes one semantic contract:

`GlorpCommand` / `GlorpQuery` -> `GlorpOutcome` / `GlorpSnapshot` / `GlorpEvent`

Stable namespaces:

- `config.*`: path-based durable config writes
- `document.*`: whole-document replacement
- `editor.*`: typed motions, mode transitions, edits, history
- `ui.*`: sidebar selection, inspect target selection, viewport scroll, pane ratio
- `scene.*`: explicit scene materialization
- `get selection` / `get inspect-details` / `get perf` / `get ui`: richer read-side projections for live automation
- `session attach` and `events *`: client/session helpers for explicit live-session control over the shared socket

See [`schema/glorp-schema.json`](/home/albert/polyrepo1/repos/glorp/schema/glorp-schema.json) for the reflection source used by CLI, Nu, and agents.
