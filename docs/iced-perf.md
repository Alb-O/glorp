# Iced Perf Notes

Performance notes from comparing this repo's custom canvas editor against cached `iced-0.14` text widgets. Check $CARGO_HOME to find cached Iced src.

## 1. Full Scene Rebuilds On Every Edit

Source: `src/app.rs`, `src/scene.rs`, Cargo cache `iced_widget-0.14.2/src/text_editor.rs`, `iced_graphics-0.14.0/src/text/editor.rs`

The main cost center is full layout rebuild on every text mutation. `refresh_scene` rebuilds the entire `LayoutScene` from the current string in `src/app.rs:270` and `src/scene.rs:32`.

`iced` keeps a persistent editor object, mutates it in place, updates bounds/font/wrap incrementally, and then calls `shape_as_needed` in `iced_widget-0.14.2/src/text_editor.rs:621` and `iced_graphics-0.14.0/src/text/editor.rs:546`.

## 2. Scroll Invalidates Cached Geometry

Source: `src/canvas_view.rs`, Cargo cache `iced_widget-0.14.2/src/text_editor.rs`, `iced_wgpu-0.14.0/src/text.rs`

Scroll currently clears `scene_cache` whenever rounded scroll changes in `src/canvas_view.rs:137`, and the scroll offset is baked into the generated geometry in `src/canvas_view.rs:173`.

`iced` text widgets keep the text buffer stable and pass clip bounds to the renderer instead of rebuilding text geometry on scroll in `iced_widget-0.14.2/src/text_editor.rs:1011` and `iced_wgpu-0.14.0/src/text.rs:617`.

## 3. `canvas::Text` Is Not The Fast Text Widget Path

Source: `src/canvas_view.rs`, Cargo cache `iced_graphics-0.14.0/src/geometry/text.rs`, `iced_widget-0.14.2/src/text_editor.rs`, `iced_widget-0.14.2/src/text_input.rs`

The canvas path uses `frame.fill_text(...)` in `src/canvas_view.rs:205`. In `iced_graphics`, `canvas::Text` allocates a paragraph and converts glyphs to paths or pixels in `iced_graphics-0.14.0/src/geometry/text.rs:48`.

Regular editable widgets use `renderer.fill_editor(...)` or `renderer.fill_paragraph(...)` with persistent buffers in `iced_widget-0.14.2/src/text_editor.rs:1011` and `iced_widget-0.14.2/src/text_input.rs:616`.

## 4. Offscreen Work Is Still Paid For

Source: `src/canvas_view.rs`, `src/scene.rs`, Cargo cache `iced_graphics-0.14.0/src/text/editor.rs`

The static canvas pass iterates all runs and glyphs in `src/canvas_view.rs:219`, and `LayoutScene::build` extracts outlines for every glyph in `src/scene.rs:57`.

`iced` avoids whole-document work in comparable paths. For example, syntax highlighting only advances through visible lines in `iced_graphics-0.14.0/src/text/editor.rs:643`.

## 5. Hit Testing And Caret Lookup Are Linear Scans

Source: `src/scene.rs`, Cargo cache `iced_widget-0.14.2/src/text_input.rs`

Hover, click, cluster lookup, and caret lookup all walk glyph or cluster collections in `src/scene.rs:207`, `src/scene.rs:265`, and `src/scene.rs:339`.

`iced` leans on paragraph/editor primitives for hit testing and grapheme positioning instead of custom scans, as seen in `iced_widget-0.14.2/src/text_input.rs:1623`.

## 6. Debug And Inspection Data Sit On The Hot Path

Source: `src/scene.rs`

`LayoutScene::build` always computes `fonts_seen`, preview strings, dump text, and optional outline command vectors in `src/scene.rs:47`.

That is useful for inspection, but it means edit-time work includes debug-oriented data production that typical `iced` text widgets do not do on every mutation.
