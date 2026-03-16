# Iced Perf Notes

This repo is no longer bottlenecked by rebuilding all text state on every edit. The important remaining costs are:

1. Width-driven text reflow during live resize.
2. Expensive derived sidebar work that still rebuilds too often.
3. The remaining split between the live editor path and the derived inspection path.

## Current Shape

The app has three runtime layers:

- `EditorBuffer` in `src/editor.rs` owns the retained `cosmic-text::Buffer`. It is the source of truth for text, motion, hit testing, and editor selection state.
- `SceneTextLayer` in `src/text_view.rs` draws the visible document through a retained `iced` paragraph.
- `LayoutScene` in `src/scene.rs` is a derived inspection snapshot used for dump/inspect/outlines, not for core editor interaction.

The canvas split is now narrower than before:

- `src/scene_view.rs` owns the retained static inspection scene cache and translates it at draw time instead of rebuilding on scroll
- `src/overlay_view.rs` draws the editor underlay and inspect overlay through normal renderer widgets
- `src/canvas_view.rs` is primarily the input/event path now

That means the visible editor path is now much closer to upstream `iced` text widgets:

- edits mutate a retained buffer instead of rebuilding from the full string
- bounds-only width changes use retained-buffer resize
- visible text rendering is on the paragraph path, not `canvas::Text`
- selection/caret overlays come from editor-owned retained-buffer geometry

## What Is Already Fixed

These are no longer the main story:

- Text edits no longer rebuild the whole `cosmic-text::Buffer`.
- The app no longer maintains two hot-path retained text buffers that both need patching after edits.
- The visible text layer no longer goes through `canvas::Text`.
- Scene clones are cheap because the heavy scene data is shared.
- Dump generation is lazy.
- `inspect_runs()` is lazy in normal text mode.
- The inspect overlay no longer lives on the canvas path.
- The editor underlay no longer lives on the canvas path.
- The static scene cache no longer invalidates on scroll.
- Editor motion and click selection no longer depend on `LayoutScene`.
- Drag resize no longer forces a full inspection rebuild for every raw size sample.

## Remaining Costs

### 1. Width Resize Is Still A Real Reflow Path

Width changes are real text-layout events. Wrapping and line breaks can change, so resize is never going to be free.

The current split is:

- the live editor/text path updates retained-buffer width immediately
- the heavier `LayoutScene` rebuild runs on the coalesced resize path
- scroll position stays stable across resize-only reflows
- `PerfMonitor` records `resize.reflow` separately from general `scene.build`

That is the right direction. It keeps divider drag responsive without forcing every derived system to keep up sample-for-sample. But the live path still pays real relayout work.

### 2. Sidebar-Derived Work Still Rebuilds Too Often

The expensive `Inspect` and `Perf` sidebar bodies are still rebuilt from the root view path.

Today that includes:

- `interaction_details()` string assembly in `src/app/view.rs`
- scene-derived target detail text
- perf dashboard assembly and graph panel rebuilds

That work is now a clearer candidate for dependency-scoped caching with `lazy(...)` or equivalent local caching, because the renderer-side canvas churn has already been pushed down substantially.

### 3. Editor And Inspect Are Still Separate Worlds

The editor path is now editor-owned, but inspect is still derived from `LayoutScene`.

Today that means:

- editor selection, drag selection, double-click word selection, caret, and selection rectangles come from `EditorBuffer`
- inspect hover/selection targeting still comes from `LayoutScene`
- inspect details in the sidebar are still scene-derived
- outline rendering still depends on eager scene data

That split is intentional, but it is still a split.

### 4. Inspect Hit Testing Is Still Custom

`src/scene.rs` still does custom hit testing over runs, clusters, and lazily built glyph inspection data.

That is acceptable for inspect mode, but it is still extra work and it still keeps inspect behavior separate from the editor’s own hit-test path.

## Next Hills

The next high-value moves are:

1. Reduce how much derived inspection work is tied to width changes.
2. Cache or defer expensive sidebar-derived views.
3. Keep pushing editor behavior out of `LayoutScene` and into `EditorBuffer`.

In practical terms:

- keep `LayoutScene` inspection-only and reduce how much it must rebuild during active resize
- stop rebuilding `Inspect` and `Perf` sidebar subtrees on unrelated view passes
- continue collapsing editor-facing behavior toward editor-owned state and geometry

The visible editor is already on the right side of that boundary. The remaining churn is mostly inspect/debug machinery.

## Perf Harness

The app already records useful runtime metrics in `src/perf.rs`:

- `editor.command`
- `editor.apply`
- `scene.build`
- `resize.reflow`
- `canvas.update`
- `canvas.static`
- `canvas.underlay`
- `canvas.overlay`
- `canvas.draw`
- frame pacing
- canvas cache hit/miss rate

Export and automation now exist in a minimal form.

### Recommended Split

Use two perf layers:

1. An in-process scripted runtime mode for end-to-end truth.
2. Cheap headless microbenchmarks for CPU-only hot paths.

The runtime mode is the more important one because the biggest remaining questions are runtime questions:

- pane-drag resize churn
- frame pacing
- static-scene cache continuity
- editor/inspect desync during resize

### Runtime Mode Shape

The repo now has a headless JSON export mode:

- `cargo run -- --perf-scenario tall-inspect --samples 180 --warmup 30`
- `cargo run -- --perf-scenario tall-perf --samples 180 --warmup 30`
- `cargo run -- --perf-scenario incremental-typing --samples 120 --warmup 30`
- `cargo run -- --perf-scenario resize-reflow --samples 60 --warmup 20`

Current scenarios are exposed by `PerfScenario` in `src/lib.rs` and split into two drivers:

- `steady-render` for static scene/render checks
- `scripted-update-render` for interleaved update/render loops like typing, motion, resize, and inspect interaction

The JSON should include:

- scenario metadata
- driver kind
- build profile
- warmup/sample counts
- avg / p95 / max by metric
- frame pacing summary
- cache hit/miss summary
- environment notes like window size and backend when available

That means the runtime mode can now answer:

- how much incremental typing costs in `editor.command` and `editor.apply`
- what coalesced resize does to `scene.build`, `resize.reflow`, and cache misses
- how inspect-mode interaction affects frame pacing when scene-derived UI is active

### Headless Benchmarks

Use headless checks for:

- edit application
- retained-buffer motion
- pointer selection logic
- full scene rebuild
- lazy `inspect_runs()` materialization
- representative command sequences in normal and insert mode

They are useful regression checks, but not a substitute for the runtime mode because they do not measure the `iced` event loop, rendering backend, scroll invalidation, or end-to-end frame pacing.
