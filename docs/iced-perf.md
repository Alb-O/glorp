# Iced Perf Notes

Performance notes from comparing this repo's custom canvas editor against cached `iced-0.14` text widgets. Check $CARGO_HOME to find cached Iced src.

Currently:

- `LayoutScene::build` no longer extracts outlines unless outline rendering is enabled.
- Dump text is now generated lazily when the Dump tab is active instead of on every scene rebuild.
- Per-glyph and per-cluster debug preview strings were removed from the hot path and are now derived lazily from `scene.text`.
- Cluster range and caret-adjacent lookups now use binary-search/`partition_point` helpers instead of repeated linear scans.
- The static canvas pass now culls runs and glyphs against the visible viewport before drawing baselines, hitboxes, and outlines.
- The document text layer now renders through a persistent `iced` paragraph widget instead of `canvas::Text`.
- Canvas scroll state is mirrored into app state so the paragraph layer and inspection overlay share one viewport.

Architectural constraint:

- `iced::widget::canvas` cache reuse is tied to the geometry built inside `Program::draw`.
- The program only receives `&Renderer`, not `&mut Renderer`, so it cannot re-apply a fresh translation to already cached geometry.
- That means true scroll-decoupled cache reuse is not available in the current canvas architecture, even though `iced` text widgets achieve it in their non-canvas renderer paths.

Smooth scrolling remains a requirement, so the animated scroll path stays in place. That means rounded scroll changes still invalidate the current canvas cache.

## 1. Full Scene Rebuilds On Every Edit

Source: `src/app.rs`, `src/scene.rs`, Cargo cache `iced_widget-0.14.2/src/text_editor.rs`, `iced_graphics-0.14.0/src/text/editor.rs`

The main cost center is full layout rebuild on every text mutation. `refresh_scene` rebuilds the entire `LayoutScene` from the current string in `src/app.rs:270` and `src/scene.rs:32`.

`iced` keeps a persistent editor object, mutates it in place, updates bounds/font/wrap incrementally, and then calls `shape_as_needed` in `iced_widget-0.14.2/src/text_editor.rs:621` and `iced_graphics-0.14.0/src/text/editor.rs:546`.

Status: still open. The current code trims rebuild cost, but text edits still rebuild the whole scene from scratch.

## 2. Scroll Still Invalidates Overlay Geometry

Source: `src/canvas_view.rs`, `src/text_view.rs`, Cargo cache `iced_widget-0.14.2/src/text_editor.rs`, `iced_wgpu-0.14.0/src/text.rs`

The overlay canvas still clears its geometry cache whenever rounded scroll changes in `src/canvas_view.rs:138`, because baselines, hitboxes, and outlines are still drawn in canvas space.

The document text no longer pays that cost. It now renders through an `iced` paragraph layer that keeps the text buffer stable and passes clip bounds to the renderer on scroll, matching the upstream text-widget path in `iced_wgpu-0.14.0/src/text.rs:617`.

Status: partially addressed. The expensive text layer is scroll-decoupled now, but the inspection overlay still invalidates on scroll because it remains a canvas pass.

## 3. The `canvas::Text` Bottleneck Is Removed

Source: `src/text_view.rs`, Cargo cache `iced_widget-0.14.2/src/text_editor.rs`, `iced_widget-0.14.2/src/text_input.rs`

The document layer now uses `renderer.fill_paragraph(...)` through a custom widget instead of `frame.fill_text(...)`.

Regular editable widgets use `renderer.fill_editor(...)` or `renderer.fill_paragraph(...)` with persistent buffers in `iced_widget-0.14.2/src/text_editor.rs:1011` and `iced_widget-0.14.2/src/text_input.rs:616`.

Status: addressed. The remaining bottlenecks are elsewhere.

## 4. Offscreen Work Is Still Paid For

Source: `src/canvas_view.rs`, `src/scene.rs`, Cargo cache `iced_graphics-0.14.0/src/text/editor.rs`

The overlay canvas still iterates visible runs and glyphs in `src/canvas_view.rs`, and `LayoutScene::build` extracts outlines for every glyph when outline rendering is enabled in `src/scene.rs:50`.

`iced` avoids whole-document work in comparable paths. For example, syntax highlighting only advances through visible lines in `iced_graphics-0.14.0/src/text/editor.rs:643`.

Status: partially addressed. Outline extraction is gated by render mode, the overlay culls vector work to the visible viewport, and the whole-document text draw has moved off canvas. The remaining offscreen cost is in the overlay/debug path and full scene rebuilds.

## 5. Hit Testing And Caret Lookup Are Linear Scans

Source: `src/scene.rs`, Cargo cache `iced_widget-0.14.2/src/text_input.rs`

Hover, click, cluster lookup, and caret lookup all walk glyph or cluster collections in `src/scene.rs:207`, `src/scene.rs:265`, and `src/scene.rs:339`.

`iced` leans on paragraph/editor primitives for hit testing and grapheme positioning instead of custom scans, as seen in `iced_widget-0.14.2/src/text_input.rs:1623`.

Status: partially addressed. Cluster range and caret-adjacent lookups now use binary-search style helpers, but hover/click hit testing still scans runs and glyphs.

## 6. Debug And Inspection Data Sit On The Hot Path

Source: `src/scene.rs`

`LayoutScene::build` always computes `fonts_seen`, preview strings, dump text, and optional outline command vectors in `src/scene.rs:47`.

That is useful for inspection, but it means edit-time work includes debug-oriented data production that typical `iced` text widgets do not do on every mutation.

Status: largely addressed. Dump generation is lazy, font reporting was reduced to a cheap count in hot paths, preview strings are now derived lazily, and outlines are only built when needed for outline rendering.

## Next Likely Step

The next meaningful improvement is to stop rebuilding the whole `LayoutScene` on every edit and instead keep a persistent layout/buffer that can update incrementally, closer to `iced_graphics::text::Editor`.
