# glorp

Simple text editor and inspection app built on `iced` and `cosmic-text`.

## Run

```bash
cargo run
```

## Repro Perf Checks

Run the representative scripted scenarios:

```bash
cargo run -- --perf-scenario large-paste
cargo run -- --perf-scenario incremental-typing
cargo run -- --perf-scenario undo-redo-burst
cargo run -- --perf-scenario delete-forward-burst
cargo run -- --perf-scenario motion-sweep
cargo run -- --perf-scenario resize-reflow
```

## Repro Trace Analysis

Use tracing to inspect where edit time goes:

```bash
GLORP_TRACE=glorp=trace cargo run -- --perf-scenario large-paste
GLORP_TRACE='glorp::editor=trace,glorp::app::update=warn' cargo run -- --perf-scenario incremental-typing 2>&1 | rg 'layout edit rebuilt full buffer|layout edit updated buffer|refresh view state|editor apply|editor command over'
GLORP_TRACE=glorp=trace cargo run -- --perf-scenario resize-reflow
```

## Current Read

- `large-paste` is dominated by the full buffer rebuild path when line structure changes.
- Incremental typing is dominated more by repeated editor snapshot and refresh work than by string cloning.
- Resize/reflow timings look reasonable for a coalesced multi-step scenario.
