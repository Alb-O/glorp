# API

Glorp now exposes one flat protocol registry:

`GlorpExec` / `GlorpQuery` / `GlorpHelper` -> `GlorpOutcome` / `GlorpQueryResult` / `GlorpHelperResult` / `GlorpEvent`

Transactions are typed exec batches:

`GlorpTxn { execs: Vec<GlorpExec> }`

The public surface is registry-driven:

- exec operations: `config-set`, `document-replace`, `editor-motion`, `scene-ensure`, and related UI/editor ops
- query operations: `schema`, `config`, `snapshot`, `selection`, `inspect-details`, `perf`, `ui`, `capabilities`
- helper operations: `session-attach`, `session-shutdown`, `config-validate`, `events-subscribe`, `events-next`, `events-unsubscribe`

The Nu plugin exposes exactly three commands:

- `glorp exec <operation> [input]`
- `glorp query <operation> [input]`
- `glorp helper <operation> [input]`

Examples:

```nu
glorp exec config-set {path: "editor.wrapping", value: "word"}
glorp query snapshot {scene: "materialize", include_document_text: true}
glorp helper session-attach
```

See `../schema/glorp-schema.json` for the generated reflection source used by the plugin and Nu completions.
