# API

Glorp now exposes one flat call protocol:

`GlorpCall` -> `GlorpCallResult` / `GlorpEvent`

The public wire format is raw-envelope based:

- `GlorpCall { id: String, input: Option<GlorpValue> }`
- `GlorpCallResult { id: String, output: GlorpValue }`

Transactions are typed mutation batches:

`GlorpTxn { calls: Vec<GlorpCall> }`

The public surface is registry-driven:

- mutation calls: `config-set`, `document-replace`, `editor-motion`, `editor-mode`, `editor-insert`, `editor-history`, and related editor/config ops
- read calls: `schema`, `config`, `document-text`, `editor`, `capabilities`
- helper/control calls: `session-attach`, `session-shutdown`, `config-validate`, `events-subscribe`, `events-next`, `events-unsubscribe`

The Nu plugin exposes one semantic command:

- `glorp call <operation> [input]`

Examples:

```nu
glorp call config-set {path: "editor.wrapping", value: "word"}
glorp call editor
glorp call session-attach
```

See `../schema/glorp-schema.json` for the generated reflection source used by the plugin and Nu completions.
Regenerate the checked-in public surface with `cargo run -p xtask -- surface`.
