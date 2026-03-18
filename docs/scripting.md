# Scripting

Two Nu-facing surfaces exist:

- [`nu/glorp.nu`](/home/albert/polyrepo1/repos/glorp/nu/glorp.nu): session-aware Nu helper module that shells through `glorp_cli`
- `glorp_nu_plugin`: direct Nu plugin commands over IPC for lower overhead and plugin-native testing

Example transcript:

```nu
use ./nu/glorp.nu *

let session = (glorp session attach)

glorp txn [
  (glorp cmd config set editor.wrapping glyph)
  (glorp cmd doc replace "hello")
  (glorp cmd editor mode enter-insert-after)
  (glorp cmd editor motion line-end)
  (glorp cmd editor edit insert " world")
] --session $session

glorp get selection --session $session
glorp get perf --session $session
```
