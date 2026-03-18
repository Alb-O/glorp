# Scripting

Two Nu-facing artifacts exist:

- `../nu/glorp.nu`: generated Nu bootstrap script that loads the plugin and completions when sourced
- `nu_plugin_glorp`: the Nushell plugin binary behind `glorp call`

Example transcript:

```nu
plugin add ./target/debug/nu_plugin_glorp
source ./nu/glorp.nu

let session = (glorp call session-attach)

glorp call txn {
  calls: [
    {id: "config-set", input: {path: "editor.wrapping", value: "glyph"}}
    {id: "document-replace", input: {text: "hello"}}
    {id: "editor-mode", input: {mode: "enter-insert-after"}}
    {id: "editor-motion", input: {motion: "line-end"}}
    {id: "editor-insert", input: {text: " world"}}
  ]
} --session $session

glorp call editor --session $session
glorp call document-text --session $session
```
