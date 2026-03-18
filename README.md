# glorp

Nushell-first text runtime with one public semantic API shared by the runtime,
IPC transport, GUI client, CLI, and Nu plugin.

## Workspace

- `crates/glorp-api`: public commands, queries, events, config, schema
- `crates/glorp-editor`: editing and layout engine
- `crates/glorp-runtime`: canonical state owner and persistence
- `crates/glorp-transport`: in-process and IPC clients
- `crates/glorp-gui`: thin GUI/client adapter
- `crates/glorp-cli`: operator and agent entrypoint
- `crates/glorp-nu-plugin`: Nu plugin commands over IPC

## Run Checks

```bash
devenv-run -C . cargo test --workspace
```

## CLI

```bash
./target/debug/glorp-cli schema
./target/debug/glorp-cli get state
./target/debug/glorp-cli config set editor.wrapping glyph
./target/debug/glorp-cli doc replace "hello"
./target/debug/glorp-cli editor mode enter-insert-after
./target/debug/glorp-cli editor motion line-end
./target/debug/glorp-cli editor edit insert " world"
./target/debug/glorp-cli scene ensure
```

## Nu

- `nu/default-config.nu`: durable data-first config
- `nu/glorp.nu`: generated readable Nu namespace that shells through the CLI
- `crates/glorp-nu-plugin`: direct Nu plugin surface over IPC

## Proof

The acceptance suite lives in [`crates/glorp-cli/tests/acceptance.rs`](/home/albert/polyrepo1/repos/glorp/crates/glorp-cli/tests/acceptance.rs).
It proves schema export, Nu round-trip, invalid config rejection, transaction
atomicity, GUI/runtime integration, scene materialization, IPC/client parity,
persistence, event stream behavior, and a golden transcript.
