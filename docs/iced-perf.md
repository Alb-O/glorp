# Iced Perf Notes

Notes from comparing this repo's custom canvas editor against cached `iced-0.14` text widgets under `$CARGO_HOME`.

The old biggest problem was rebuilding too much scene state on every edit. That is no longer the main story.

## Current Shape

The app now has three distinct layers:

- A retained `cosmic-text::Buffer` owned by `EditorBuffer` in `src/editor.rs` is the source of truth for text layout and edit shaping.
- The visible document text is drawn through a retained `iced` paragraph in `src/text_view.rs`, not through `canvas::Text`.
- `LayoutScene` in `src/scene.rs` is now a derived inspection/layout snapshot rebuilt from that retained buffer.
- The inspection overlay still lives in `iced::widget::canvas` in `src/canvas_view.rs`.

That puts the text path much closer to upstream `iced` widget behavior:

- `iced_widget-0.14.2/src/text_editor.rs` keeps a retained editor object and updates it incrementally.
- `iced_graphics-0.14.0/src/text/editor.rs` does incremental buffer/editor updates and then `shape_as_needed(...)`.
- `iced_wgpu-0.14.0/src/text.rs` applies transform and clip to retained text areas instead of rebuilding shaped text on scroll.

## What Is Already Fixed

These are no longer the main bottlenecks:

- Text edits no longer rebuild the `cosmic-text::Buffer` from the full string. `EditorBuffer` keeps the retained buffer and applies edits in place in `src/editor.rs`.
- The app no longer patches two separate retained text buffers after every edit. `LayoutSceneModel` rebuilds from the editor-owned buffer in `src/app.rs` and `src/scene.rs`.
- The document text layer no longer uses `canvas::Text`. It uses `renderer.fill_paragraph(...)` in `src/text_view.rs`.
- Scene clones are cheap. The shared text, runs, clusters, warnings, and inspection cache backing are all reference-counted in `src/scene.rs`.
- Dump generation is lazy and only happens when the Dump tab is active in `src/app.rs`.
- Cluster range lookup and several caret-adjacent lookups use binary-search style helpers instead of repeated linear scans in `src/scene.rs`.
- Inspection glyph data is no longer materialized eagerly on every edit in normal text mode. `LayoutScene::from_buffer(...)` now builds lightweight runs/clusters first, and `inspect_runs()` only constructs per-glyph inspection payload on demand unless outline mode requires it eagerly in `src/scene.rs`.
- The canvas draw pass culls vector work to the visible viewport in `src/canvas_view.rs`.

## Remaining Structural Costs

### 1. Canvas Scroll Still Invalidates Overlay Cache

The overlay cache still clears whenever rounded scroll changes in `src/canvas_view.rs`.

That is expected with the current canvas architecture:

- `iced::widget::canvas` cache reuse is tied to geometry built inside `Program::draw`.
- The canvas program only gets `&Renderer`, not `&mut Renderer`.
- So the overlay cannot cheaply re-translate already cached geometry the way the non-canvas text path can.

The expensive text layer is already off this path. What still pays here is overlay/debug geometry.

### 2. Editor Interaction Is Still Scene-Driven

This is now the biggest hill.

The app still keeps:

- a retained `cosmic-text::Buffer` in `src/editor.rs`
- a custom normal/insert interaction model in `src/editor.rs`
- a synchronization step after each scene refresh in `src/app.rs`

The retained text buffer is now shared, but most cursor behavior is still derived from `LayoutScene` clusters instead of the retained text core:

- click selection still goes through `scene.hit_test_cluster(...)`
- vertical movement and line-edge movement still use `scene.nearest_cluster_on_adjacent_run(...)`, `scene.first_cluster_in_run(...)`, `scene.last_cluster_in_run(...)`, and `scene.caret_metrics(...)`
- normal-mode selection is still stored as a scene cluster byte range and repaired by `sync_with_scene(...)` after each scene refresh

Upstream `iced` does not route those paths through a derived scene snapshot. Its text editor path keeps one retained editor/buffer object and drives motion, selection, hit testing, shaping, and rendering from the same core state in `iced_widget-0.14.2/src/text_editor.rs`, `iced_graphics-0.14.0/src/text/editor.rs`, and `cosmic-text` itself.

### 3. Hit Testing Is Still Custom And Linear

`src/scene.rs` still does custom hover/click hit testing over runs, clusters, or lazily built glyph inspection data.

This is much less important than the old whole-document rebuild issue, but it still means:

- first glyph-target hover in non-outline mode can trigger lazy inspection-glyph construction
- pointer interactions still walk custom scene data instead of using a native editor/paragraph hit-test primitive

## What Changed Most Recently

The current editor/scene split is the important shift:

- `EditorBuffer` owns the retained `cosmic-text::Buffer`
- hot-path scene refresh now rebuilds only the derived inspection/layout snapshot from that retained buffer
- glyph inspection runs are backed by the retained buffer and built lazily through `inspect_runs()`
- outline mode remains eager because the canvas outline renderer genuinely needs outline paths immediately

That means the old advice, "stop patching two retained text buffers and stop materializing whole-document glyph snapshots on every edit," is now implemented.

## Next Big Hill

The next big hill is to stop deriving editor interaction from `LayoutScene`.

In practice that means pushing toward one retained editor core that owns:

- caret
- selection
- line movement
- edit application
- hit testing or hit-test inputs

and then deriving the inspection view from that, instead of using scene clusters as the state machine for cursor movement and selection.

The concrete next slice is:

- switch click selection from `scene.hit_test_cluster(...)` to retained-buffer hit testing
- switch `MoveUp`, `MoveDown`, `MoveLineStart`, and `MoveLineEnd` from scene helpers to retained-buffer cursor motion
- treat normal-mode cluster highlighting as a derived projection of the retained cursor, not as the cursor source of truth
- shrink or remove `sync_with_scene(...)` so scene refresh stops "repairing" editor state after the fact

That is the highest-value next move because it should improve both:

- correctness: cursor desync, stale line movement, and scene/editor mismatch bugs
- performance: less post-refresh reconciliation and less custom scene-driven editor logic

If that is too large for one pass, the next-best pure perf project is to move more of the inspection overlay off `canvas` so scroll no longer forces cached geometry rebuilds for the overlay path.

## Automated CLI Perf Metrics And Tests

We want automated CLI perf checks that stay close to real runtime behavior.

Today the app already records useful timings in `src/perf.rs`:

- editor command/apply
- scene build
- canvas update/static/overlay/draw
- frame pacing
- canvas cache hit/miss rate

The missing piece is export and automation. Those metrics only exist inside the interactive UI today.

### Recommendation

Use a two-tier setup:

1. An in-process scripted runtime perf mode for end-to-end truth.
2. Small headless microbenchmarks for CPU-only hot paths.

That split matches the code:

- `scene.rs` and `editor.rs` are good headless benchmark targets.
- `canvas_view.rs`, `text_view.rs`, cache invalidation, and frame pacing need the real `iced` runtime.

### What To Build First

Build the scripted runtime mode first.

That is the best first move because:

- the app already has `PerfMonitor`
- the main open questions are runtime questions: scroll invalidation, canvas cost, and editor/runtime desync
- those questions need end-to-end data, not just isolated CPU timings

External GUI automation is not the right first tool here. It is slower, noisier, and less deterministic than an in-process harness.

### Runtime Harness Shape

Add a perf CLI mode that launches the normal app, runs a fixed scenario, prints JSON, and exits.

A reasonable shape is:

- `liney --perf-scenario edit-tall --samples 200 --warmup 30`
- `liney --perf-scenario scroll-text --samples 300`
- `liney --perf-scenario scroll-outlines --samples 300`

Each scenario should pin:

- preset
- font
- shaping
- wrapping
- render mode
- layout width
- window size

and then drive the same sequence of app messages every run.

The JSON output should include:

- scenario metadata
- build profile
- sample and warmup counts
- avg / p95 / max for each existing metric
- frame pacing summary
- cache hit/miss summary
- environment notes such as platform, window size, and renderer/backend when available

### Headless Benchmarks

Add a second layer of cheap deterministic measurements around:

- `EditorBuffer` edit application
- retained-buffer motion and click-selection paths
- full scene rebuild
- lazy `inspect_runs()` materialization
- representative editor command sequences in both insert and normal mode

These will be useful for frequent regression checks, but they are not a substitute for the runtime harness because they do not measure:

- the `iced` event loop
- `wgpu` text rendering
- canvas cache invalidation on scroll
- end-to-end frame pacing

### How To Use It

Use headless perf checks as the fast regression layer. Use the scripted runtime command as the realistic layer.

The runtime harness should be:

- easy to run locally during perf work
- optional in normal CI
- run in a dedicated perf job or scheduled run on stable hardware when numbers need to be comparable

Do not use generic `cargo test` wall-clock assertions as the main perf gate. They are too sensitive to host noise, display setup, and font differences.
