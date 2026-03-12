# Iced Perf Notes

This repo is no longer bottlenecked by rebuilding all text state on every edit. The important remaining costs are:

1. Canvas overlay cache invalidation on scroll.
2. Width-driven text reflow during live resize.
3. The remaining split between the live editor path and the derived inspection path.

## Current Shape

The app has three runtime layers:

- `EditorBuffer` in `src/editor.rs` owns the retained `cosmic-text::Buffer`. It is the source of truth for text, motion, hit testing, and editor selection state.
- `SceneTextLayer` in `src/text_view.rs` draws the visible document through a retained `iced` paragraph.
- `LayoutScene` in `src/scene.rs` is a derived inspection snapshot used for dump/inspect/outlines, not for core editor interaction.

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
- The canvas draw pass culls vector work to the visible viewport.
- Editor motion and click selection no longer depend on `LayoutScene`.
- Drag resize no longer forces a full inspection rebuild for every raw size sample.

## Remaining Costs

### 1. Canvas Overlay Cache Still Rebuilds On Scroll

The inspection/debug overlay still lives in `iced::widget::canvas` in `src/canvas_view.rs`.

That means:

- rounded scroll changes still invalidate the canvas cache
- overlay/debug geometry still rebuilds on scroll
- the non-canvas text path avoids this cost, but inspect overlays do not

This is still the clearest draw-time cost.

### 2. Width Resize Is Still A Real Reflow Path

Width changes are real text-layout events. Wrapping and line breaks can change, so resize is never going to be free.

The current split is:

- the live editor/text path updates retained-buffer width immediately
- the heavier `LayoutScene` rebuild runs on the coalesced resize path
- scroll position stays stable across resize-only reflows
- `PerfMonitor` records `resize.reflow` separately from general `scene.build`

That is the right direction. It keeps divider drag responsive without forcing every derived system to keep up sample-for-sample. But the live path still pays real relayout work.

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

1. Shrink the inspection overlay cost.
2. Reduce how much derived inspection work is tied to width changes.
3. Keep pushing editor behavior out of `LayoutScene` and into `EditorBuffer`.

In practical terms:

- move more inspect rendering off `iced::widget::canvas`, or otherwise stop scroll from forcing so much overlay rebuild work
- keep `LayoutScene` inspection-only and reduce how much it must rebuild during active resize
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
- `canvas.overlay`
- `canvas.draw`
- frame pacing
- canvas cache hit/miss rate

What is still missing is export and automation.

### Recommended Split

Use two perf layers:

1. An in-process scripted runtime mode for end-to-end truth.
2. Cheap headless microbenchmarks for CPU-only hot paths.

The runtime mode is the more important one because the biggest remaining questions are runtime questions:

- scroll invalidation
- canvas overlay cost
- pane-drag resize churn
- frame pacing
- editor/inspect desync during resize

### Runtime Mode Shape

Add a CLI mode that launches the normal app, runs a fixed scenario, prints JSON, and exits.

Example shapes:

- `liney --perf-scenario edit-tall --samples 200 --warmup 30`
- `liney --perf-scenario scroll-text --samples 300`
- `liney --perf-scenario scroll-outlines --samples 300`
- `liney --perf-scenario resize-pane --samples 180`

Each scenario should pin:

- preset
- font
- shaping
- wrapping
- render mode
- layout width
- window size

The JSON should include:

- scenario metadata
- build profile
- warmup/sample counts
- avg / p95 / max by metric
- frame pacing summary
- cache hit/miss summary
- environment notes like platform, window size, and backend when available

### Headless Benchmarks

Use headless checks for:

- edit application
- retained-buffer motion
- pointer selection logic
- full scene rebuild
- lazy `inspect_runs()` materialization
- representative command sequences in normal and insert mode

They are useful regression checks, but not a substitute for the runtime mode because they do not measure the `iced` event loop, rendering backend, scroll invalidation, or end-to-end frame pacing.
