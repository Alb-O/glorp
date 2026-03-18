# Scripting

Two Nu-facing artifacts exist:

- `../nu/glorp.nu`: generated Nu bootstrap script that loads the plugin and completions when sourced
- `nu_plugin_glorp`: the Nushell plugin binary behind `glorp exec`, `glorp query`, and `glorp helper`

Example transcript:

```nu
plugin add ./target/debug/nu_plugin_glorp
source ./nu/glorp.nu

let session = (glorp helper session-attach)

glorp exec txn {
  execs: [
    {op: "config-set", input: {path: "editor.wrapping", value: "glyph"}}
    {op: "document-replace", input: {text: "hello"}}
    {op: "editor-mode", input: {mode: "enter-insert-after"}}
    {op: "editor-motion", input: {motion: "line-end"}}
    {op: "editor-insert", input: {text: " world"}}
  ]
} --session $session

glorp query editor --session $session
glorp query document-text --session $session
```
