# glorp

Nushell-first text runtime with one public semantic API shared by the runtime,
IPC transport, GUI client, CLI, and Nu plugin.

## Workspace

- `crates/api`: public commands, queries, events, config, schema
- `crates/editor`: editing and layout engine
- `crates/runtime`: canonical state owner and persistence
- `crates/transport`: in-process and IPC clients
- `crates/gui`: thin GUI/client adapter
- `crates/cli`: operator and agent entrypoint
- `crates/nu-plugin`: Nu plugin commands over IPC

## Run Checks

```sh
devenv-run -C . cargo test --workspace
```

## CLI

```sh
./target/debug/glorp_cli schema
./target/debug/glorp_cli get state
./target/debug/glorp_cli config set editor.wrapping glyph
./target/debug/glorp_cli doc replace "hello"
./target/debug/glorp_cli editor mode enter-insert-after
./target/debug/glorp_cli editor motion line-end
./target/debug/glorp_cli editor edit insert " world"
./target/debug/glorp_cli scene ensure
```

## GUI

```sh
devenv-run -C . cargo run -p glorp_gui
```

The GUI hosts or joins the shared runtime on `./glorp.sock`. When that socket is live,
`glorp_cli` auto-attaches to it from the repo root instead of creating a private local runtime.

## Nu

- `nu/default-config.nu`: durable data-first config
- `nu/glorp.nu`: generated readable Nu namespace that shells through the CLI
- `crates/nu-plugin`: direct Nu plugin surface over IPC

## Proof

The acceptance suite lives in [`crates/cli/tests/acceptance.rs`](/home/albert/polyrepo1/repos/glorp/crates/cli/tests/acceptance.rs).
It proves schema export, Nu round-trip, invalid config rejection, transaction
atomicity, GUI/runtime integration, scene materialization, IPC/client parity,
persistence, event stream behavior, the GUI socket contract, and a golden transcript.
