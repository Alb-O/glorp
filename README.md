# glorp

Nushell-first text runtime with one public semantic API shared by the runtime,
IPC transport, GUI client, Nu plugin, and a minimal shared-runtime host.

## Workspace

- `crates/api`: public commands, queries, events, config, schema
- `crates/editor`: editing and layout engine
- `crates/runtime`: canonical state owner and persistence
- `crates/transport`: in-process and IPC clients
- `crates/gui`: thin GUI/client adapter
- `crates/cli`: shared runtime host and surface export tool
- `crates/nu-plugin`: Nu plugin commands over IPC

## Run Checks

```sh
devenv-run -C . cargo test --workspace
```

## Host

```sh
devenv-run -C . cargo run -p glorp_host
devenv-run -C . cargo run -p glorp_host -- export-surface
```

## GUI

```sh
devenv-run -C . cargo run -p glorp_gui
```

The GUI hosts or joins the shared runtime on `./glorp.sock`. When that socket is live,
Nu/plugin commands attach to it automatically from the repo root, and start `glorp_host`
when a shared runtime is not already live.

## Nu

- `nu/default-config.nu`: durable data-first config
- `nu/glorp.nu`: generated Nu helper module with txn builders and aliases
- `crates/nu-plugin`: primary Nu command surface over the shared runtime

## Proof

The acceptance suite lives in crates/cli/tests/acceptance.rs.
It proves schema export, Nu round-trip, invalid config rejection, transaction
atomicity, GUI/runtime integration, scene materialization, IPC/plugin parity,
persistence, event stream behavior, host auto-start, the GUI socket contract, and generated artifact drift.
