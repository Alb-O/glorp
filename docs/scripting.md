# Scripting

Two Nu-facing surfaces exist:

- [`nu/glorp.nu`](/home/albert/polyrepo1/repos/glorp/nu/glorp.nu): readable generated module that shells through `glorp_cli`
- `glorp_nu_plugin`: direct Nu plugin commands over IPC for lower overhead and plugin-native testing

Example transcript:

```nu
use ./nu/glorp.nu *

glorp config set editor.wrapping glyph
glorp doc replace "hello"
glorp editor mode enter-insert-after
glorp editor motion line-end
glorp editor edit insert " world"
glorp scene ensure
glorp get state
```
