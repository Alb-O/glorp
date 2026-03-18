# Scripting

Two Nu-facing surfaces exist:

- ../nu/glorp.nu: generated Nu helper module with txn builders and aliases
- `glorp_nu_plugin`: primary Nu command surface over the shared runtime

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
