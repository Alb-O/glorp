# Runtime Perf Snapshot

- Date: 2026-03-16
- Commit: `0b659f6`
- Build: `debug`
- Renderer: `wgpu`
- Command shape: `cargo run -q -- --perf-scenario <scenario> --warmup <n> --samples <n>`

## Scenarios

### Steady Render

`tall-perf` (`--warmup 10 --samples 30`)

- frame avg: `4.02 ms`
- frame max: `5.03 ms`
- fps: `248.8`
- cache misses: `0 / 30`

`tall-inspect` (`--warmup 10 --samples 30`)

- frame avg: `3.43 ms`
- frame max: `5.00 ms`
- fps: `291.7`
- cache misses: `0 / 30`

### Scripted Update + Render

`resize-reflow` (`--warmup 5 --samples 16`)

- frame avg: `18.14 ms`
- frame max: `20.50 ms`
- `scene.build` avg: `2.58 ms`
- `resize.reflow` avg: `2.58 ms`
- cache misses: `16 / 16`

`inspect-interaction` (`--warmup 5 --samples 18`)

- frame avg: `7.20 ms`
- frame max: `13.64 ms`
- `editor.command` avg: `2.26 ms`
- `editor.apply` avg: `2.25 ms`
- cache misses: `0 / 18`

`incremental-typing` (`--warmup 5 --samples 20`)

- frame avg: `788.13 ms`
- frame max: `805.63 ms`
- `editor.command` avg: `3.61 ms`
- `editor.apply` avg: `3.59 ms`
- cache misses: `0 / 20`

## Read

- Steady-state render looks healthy. Both static inspect and perf tabs stay well under frame budget with zero cache misses.
- Resize is the clearest remaining runtime hill. Reflow itself is only about `2.6 ms`, but the full update + render loop lands around `18 ms` and misses frame budget frequently.
- Inspect interaction is in decent shape. Interaction-driven editor work is around `2.3 ms`, and total frame time stays below budget in this headless run.
- Incremental typing shows a split result: editor mutation itself is cheap, but end-to-end headless update + render on the tall document is still extremely slow. In this scenario the bottleneck is not `editor.apply`; it is the full frame path around it.

## Caveats

- These are headless runtime snapshots, not release benchmarks.
- The JSON runner includes screenshot capture in the sampled frame loop, so frame pacing here is useful for relative comparisons, not absolute user-facing FPS claims.
- `incremental-typing` and `resize-reflow` operate on the large scripted headless document, while `tall-*` scenarios use the smaller preset document. Compare within scenario families, not across them.
