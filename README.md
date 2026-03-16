# glorp

Text layout and editor playground built on `iced` and `cosmic-text`.

## Run

```bash
cargo run
```

## Repro Perf Checks

The headless Criterion bench defaults are already tuned for quick scripted runs.

Build the bench binary once:

```bash
cargo bench --bench headless --no-run
```

Run the representative scripted scenarios:

```bash
cargo bench --bench headless -- --noplot 'playground/headless-script/large-paste'
cargo bench --bench headless -- --noplot 'playground/headless-script/incremental-typing'
cargo bench --bench headless -- --noplot 'playground/headless-script/undo-redo-burst'
cargo bench --bench headless -- --noplot 'playground/headless-script/delete-forward-burst'
cargo bench --bench headless -- --noplot 'playground/headless-script/motion-sweep'
cargo bench --bench headless -- --noplot 'playground/headless-script/resize-reflow-sweep'
```

## Repro Trace Analysis

Use tracing to inspect where edit time goes:

```bash
GLORP_TRACE=glorp=trace cargo bench --bench headless -- --noplot 'playground/headless-script/large-paste'
GLORP_TRACE='glorp::editor=trace,glorp::app::update=warn' cargo bench --bench headless -- --noplot 'playground/headless-script/incremental-typing' 2>&1 | rg 'apply buffer edit|layout edit updated retained buffer|layout edit rebuilt full buffer|refresh view state|editor apply|editor command over'
GLORP_TRACE=glorp=trace cargo bench --bench headless -- --noplot 'playground/headless-script/resize-reflow-sweep'
```

## Current Read

- `large-paste` is dominated by the full buffer rebuild path when line structure changes.
- Incremental typing is dominated more by repeated editor snapshot and refresh work than by string cloning.
- Resize/reflow timings look reasonable for a coalesced multi-step scenario.
