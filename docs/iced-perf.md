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
- `iced_graphics-0.14.0/src/text/paragraph.rs` also keeps a retained paragraph and treats bounds-only changes as `Paragraph::resize(...)`, which calls `Buffer::set_size(...)` instead of rebuilding the whole paragraph text payload.

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
- Editor interaction no longer depends on `LayoutScene` as its state machine. Click selection now starts from retained-buffer hit testing, vertical and line-edge movement derive from retained-buffer layout, and scene refresh no longer repairs editor state after the fact in `src/editor.rs` and `src/app.rs`.
- Drag-resize no longer rebuilds on every raw size sample. The app now updates retained-buffer width immediately for the live text/editor path, coalesces the heavier inspection-scene rebuild to roughly frame cadence, and preserves canvas scroll across resize-only reflows in `src/app.rs`.
- Editor selection and caret overlays now come from editor-owned geometry cached from the retained buffer layout in `src/editor.rs`, so overlay positioning follows live width changes without waiting for `LayoutScene` rebuild.

## Remaining Structural Costs

### 1. Canvas Scroll Still Invalidates Overlay Cache

The overlay cache still clears whenever rounded scroll changes in `src/canvas_view.rs`.

That is expected with the current canvas architecture:

- `iced::widget::canvas` cache reuse is tied to geometry built inside `Program::draw`.
- The canvas program only gets `&Renderer`, not `&mut Renderer`.
- So the overlay cannot cheaply re-translate already cached geometry the way the non-canvas text path can.

The expensive text layer is already off this path. What still pays here is overlay/debug geometry.

### 2. Width Resize Is Inherently A Reflow Path

The newer fullscreen editor shell exposed a different hot path: width changes.

The important upstream behavior here is:

- `iced_widget-0.14.2/src/pane_grid.rs` publishes `ResizeEvent` continuously while the divider is dragged. This is not a one-shot "resize finished" signal; it is per cursor move.
- `iced_widget-0.14.2/src/sensor.rs` publishes `on_resize` whenever the child layout size changes on redraw.
- `iced_graphics-0.14.0/src/text/paragraph.rs` handles bounds-only changes via `Paragraph::resize(...)`, which calls `buffer.set_size(...)` and then realigns.
- `iced_graphics-0.14.0/src/text/editor.rs` likewise handles editor bounds changes by calling `buffer.set_size(...)` and then `shape_as_needed(...)`.

That means upstream `iced` is already better than "throw away the paragraph/editor and rebuild from the full string", but width drag is still not free. A width change is a real text-layout event, because wrapping and line breaks can change.

Our current repo behavior now reflects that distinction:

- the visible text layer is already on the retained-paragraph path and follows live width immediately
- the editor buffer also applies width-only resize through `Buffer::set_size(...)` instead of full buffer rebuild
- editor selection/caret overlays follow cached retained-buffer geometry during drag
- the app records resize work separately as `resize.reflow` in `src/perf.rs`
- the resize path is coalesced to frame cadence and keeps scroll position stable in `src/app.rs`

That means the resize path is now split in a more useful way:

- the user-visible text/editor path pays the real relayout cost immediately
- the heavier derived inspection snapshot is allowed to lag behind at a lower cadence

That still does not make width drag free. It just stops tying every derived subsystem to every raw drag sample.

So the deeper follow-on is not "make resize cheap with better throttling." The deeper follow-on is to reduce how much state is tied to wrapping width in the first place, or to defer expensive derived work while the width is still in motion.

### 3. Editor Core Is Better, But Still Split Across Two Snapshots

The editor no longer depends on `LayoutScene` for interaction, and the visible selection/caret overlays no longer depend on it either. But the app still keeps a custom normal/insert model in `src/editor.rs`, and inspection still comes from a separate derived snapshot in `src/scene.rs`.

That means:

- click selection starts from `Buffer::hit(...)`, but normal-mode cluster choice and vertical movement are still implemented in repo-local code
- caret and normal-mode selection are editor-owned byte positions and byte ranges, with editor-owned cached geometry for the live overlay path
- the retained `cosmic-text::Buffer` is shared with `LayoutScene` through `Arc`, so a naive switch to `cosmic_text::Editor` cursor motion would be wrong here: its motion path needs `&mut Buffer`, and in this architecture that would force `Arc::make_mut(...)` and risk cloning the whole buffer on cursor movement
- for that reason, the current unification uses read-only retained-buffer layout snapshots for motion, hit-test projection, and live editor overlay geometry, while keeping actual buffer mutation limited to real text edits
- inspection hover/selection targeting still comes from `LayoutScene`, so there is still a split between the live editor path and the derived inspection path during active resize
- upstream `iced` still has a tighter one-object model where motion, cursor state, selection state, and rendering all sit behind the same retained editor abstraction

This is much better than the old scene-driven split, but it is still not the same architecture as upstream `iced_widget-0.14.2/src/text_editor.rs` plus `iced_graphics-0.14.0/src/text/editor.rs`.

### 4. Hit Testing Is Still Custom And Linear In The Overlay

`src/scene.rs` still does custom hover/click hit testing over runs, clusters, or lazily built glyph inspection data.

This is much less important than the old whole-document rebuild issue, but it still means:

- first glyph-target hover in non-outline mode can trigger lazy inspection-glyph construction
- pointer interactions still walk custom scene data instead of using a native editor/paragraph hit-test primitive

## What Changed Most Recently

The current editor/scene split is the important shift:

- `EditorBuffer` owns the retained `cosmic-text::Buffer`
- hot-path scene refresh now rebuilds only the derived inspection/layout snapshot from that retained buffer
- editor interaction now derives from retained-buffer hit testing and retained-buffer layout instead of `LayoutScene`
- editor selection and caret overlay geometry are cached from that retained-buffer layout in `EditorBuffer`
- that retained-buffer layout path is intentional: it avoids per-motion `Arc::make_mut(...)` pressure that would come from driving cursor movement through a mutable `cosmic_text::Editor` while the buffer is also shared with scene state
- glyph inspection runs are backed by the retained buffer and built lazily through `inspect_runs()`
- outline mode remains eager because the canvas outline renderer genuinely needs outline paths immediately

That means the old advice, "stop patching two retained text buffers and stop materializing whole-document glyph snapshots on every edit," is now implemented.

The newest resize-specific change is more structural:

- width now follows the live canvas viewport instead of a manual slider
- raw resize samples update retained-buffer width immediately in `src/app.rs` and `src/editor.rs`
- the heavy `LayoutScene` rebuild still runs on the coalesced resize path in `src/app.rs`
- selection/caret overlay geometry follows the retained-buffer width immediately in `src/editor.rs` and `src/canvas_view.rs`
- resize reflow preserves scroll instead of resetting the canvas viewport
- Perf now reports `resize.reflow` separately from general `scene.build`

That improves drag behavior and makes the remaining cost more isolated: width changes still drive retained-buffer relayout, but they no longer force the full inspection path to keep up sample-for-sample just to keep text and caret motion looking responsive.

## Next Big Hill

There are now two real hills:

1. The inspection overlay path.
2. Width-driven reflow during editor resize.

The overlay path is still the clearest draw-time cost.

The resize path is now the clearest layout-time cost.

In practice that means:

- moving more of the overlay off `iced::widget::canvas`, or otherwise restructuring it so scroll stops forcing cached geometry rebuilds in `src/canvas_view.rs`
- reducing the amount of derived inspection work that still rebuilds on width change in `src/app.rs` and `src/scene.rs`
- shrinking the remaining live-vs-derived mismatch during active resize, especially hover/selection details that still come from `LayoutScene`

Those are the highest-value next moves because:

- the main text layer is already off canvas
- editor interaction and editor overlays no longer depend on scene rebuild and repair
- the remaining obvious runtime churn is overlay cache invalidation on scroll plus retained-buffer relayout on width drag

The next-best follow-on after that is deeper editor unification:

- collapse normal-mode selection and insert caret toward one retained cursor/selection model
- reuse more upstream `cosmic-text` or `iced` editor semantics instead of repo-local motion helpers

## Automated CLI Perf Metrics And Tests

We want automated CLI perf checks that stay close to real runtime behavior.

Today the app already records useful timings in `src/perf.rs`:

- editor command/apply
- scene build
- resize reflow
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
- the main open questions are runtime questions: scroll invalidation, canvas cost, width-drag reflow, and editor/runtime desync
- those questions need end-to-end data, not just isolated CPU timings

External GUI automation is not the right first tool here. It is slower, noisier, and less deterministic than an in-process harness.

### Runtime Harness Shape

Add a perf CLI mode that launches the normal app, runs a fixed scenario, prints JSON, and exits.

A reasonable shape is:

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
- pane drag event frequency and viewport resize churn
- end-to-end frame pacing

### How To Use It

Use headless perf checks as the fast regression layer. Use the scripted runtime command as the realistic layer.

The runtime harness should be:

- easy to run locally during perf work
- optional in normal CI
- run in a dedicated perf job or scheduled run on stable hardware when numbers need to be comparable

Do not use generic `cargo test` wall-clock assertions as the main perf gate. They are too sensitive to host noise, display setup, and font differences.
