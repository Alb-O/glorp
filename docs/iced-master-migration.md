# Iced Master Migration Notes

These notes scope a migration from `iced = 0.14.0` to `iced` master (`0.15.0-dev`) and include the follow-up simplification that becomes possible once both `glorp` and `iced` use `cosmic-text 0.18`.

Checked upstream revision:

- `iced` master: `54020d75564eb411de0327d361c739eaed1ac41f`

Checked upstream facts at that revision:

- workspace `cosmic-text = "0.18"`
- `rust-version = "1.92"`
- `pick_list` helper signature changed
- `Font::with_name` is gone
- `Theme::extended_palette()` is gone
- headless renderer construction uses `renderer::Settings`
- manual `Text { ... }` initializers now require `ellipsis` and `hint_factor`

## Why This Migration Matters

The immediate dependency bump is not the main value. The main value is that `glorp` can stop maintaining a renderer-only compatibility buffer in order to bridge `cosmic-text 0.18` in the app with `cosmic-text 0.15` inside `iced 0.14`.

Today the visible text path in `src/editor/layout_state.rs` owns:

- the real editor buffer on the app side
- a second retained raw render buffer for `iced`

Once `iced` also uses `cosmic-text 0.18`, the second buffer should become unnecessary.

## Scratch Upgrade Findings

A scratch copy of the repo was pointed at `iced` master and checked with Cargo.

### First blocker

The first blocker was not app code. It was dependency resolution:

- current `criterion` / `plotters` stack wanted `web-sys 0.3.91`
- `iced` master currently pulled `web-sys 0.3.85`

That means a naive dependency swap can fail at resolution time before compile errors are visible.

In the scratch check, removing the bench-only `criterion` dependency was enough to reach app code and get a real compile surface.

### App-code compile surface

The scratch `cargo check --lib` reached app code and failed with 48 compile errors.

Those errors mostly collapse into these migration buckets:

1. `pick_list` helper signature changed.
2. theme palette access changed.
3. font construction changed.
4. headless renderer initialization changed.
5. manual text struct initialization changed.

That is a medium migration, not a rewrite.

## Direct Migration Tasks

### 1. Update `pick_list` call sites

Current code in `src/ui/components/controls.rs` assumes the old helper signature:

- current shape: `pick_list(options, selected, on_select)`
- master shape: `pick_list(selected, options, to_string)`

Implications:

- reorder arguments at every call site
- map from borrowed option values in closures
- likely dereference values inside message constructors
- re-check style builder chaining after the type change

Primary files:

- `src/ui/components/controls.rs`

### 2. Replace `extended_palette()` usage

`Theme::extended_palette()` is gone on master. Existing UI code uses it heavily for color derivation.

Implications:

- migrate to `theme.palette()`
- re-map any usages that depended on richer derived palette groupings
- replace references to `iced::theme::palette::Extended`

Primary files:

- `src/ui/tokens.rs`
- `src/ui/components/inspect.rs`
- `src/ui/components/perf.rs`

This is the largest visible migration slice.

### 3. Replace `Font::with_name(...)`

Master exposes `Font::new(...)` and `Font::with_family(...)` instead.

Implications:

- replace `Font::with_name("...")`
- prefer `Font::new("...")` for static named fonts
- or `Font::with_family("...")` where family coercion is clearer

Primary files:

- `src/types.rs`
- `src/lib.rs`

### 4. Update headless renderer setup

The headless renderer constructor changed from positional font/size arguments to a `renderer::Settings` struct.

Implications:

- create `iced::advanced::renderer::Settings`
- set `default_font`
- set `default_text_size`
- keep backend selection unchanged

Primary files:

- `src/headless_perf.rs`

### 5. Add new fields to manual `Text` initializers

Manual construction of `iced` text structs now requires:

- `ellipsis`
- `hint_factor`

Primary files:

- `src/overlay_view.rs`

This is a small, mechanical fix.

## Expected Migration Order

The least risky order is:

01. Solve dependency-resolution issues in the branch.
02. Point `iced` and `iced_runtime` to master.
03. Fix font constructor changes.
04. Fix `pick_list` call sites.
05. Fix palette API usage.
06. Fix headless renderer construction.
07. Fix manual `Text` initializers.
08. Run `cargo check`.
09. Run `cargo test --lib`.
10. Run the headless perf scenarios used as regression checks.

## Simplification After Shared `cosmic-text 0.18`

Once `iced` and the app share `cosmic-text 0.18`, the compatibility rendering layer should be simplified.

### Current compatibility state

In `src/editor/layout_state.rs`, `EditorLayout` currently owns:

- `buffer: Arc<cosmic_text::Buffer>`
- `render_buffer: Arc<iced::advanced::graphics::text::cosmic_text::Buffer>`

It also owns compatibility glue:

- `build_render_buffer(...)`
- `resize_render_buffer(...)`
- `apply_render_buffer_edit(...)`
- byte-to-line/column conversion for edit replay

That code exists only because `iced 0.14` and the app use different `cosmic-text` versions.

### Removal target

After the upgrade, remove the duplicated renderer buffer and render directly from the editor-owned buffer.

That means:

1. Delete `render_buffer` from `EditorLayout`.
2. Delete `build_render_buffer(...)`.
3. Delete `resize_render_buffer(...)`.
4. Delete `apply_render_buffer_edit(...)`.
5. Remove compatibility-only byte mapping that is only needed for render-buffer replay.
6. Change `EditorTextLayerState` to carry a weak handle to the real editor buffer instead of a compatibility buffer.
7. Keep `SceneTextLayer` drawing through `fill_raw(...)`, but point it at the real editor buffer.

### Files to simplify

- `src/editor/layout_state.rs`
- `src/editor/mod.rs`
- `src/editor/text.rs`
- `src/text_view.rs`

### Expected wins from removal

- lower memory use for large documents
- one text mutation per edit instead of two
- one width/config sync path instead of two
- less risk of drift between visible text and editor geometry
- less code in the text path
- easier future maintenance around shaping/wrapping behavior

## Verification Plan

After the direct migration and after the compatibility-layer removal, re-run:

1. `cargo check`
2. `cargo test --lib`
3. `cargo run -q -- --perf-scenario incremental-typing --warmup 5 --samples 20`
4. `cargo run -q -- --perf-scenario resize-reflow --warmup 5 --samples 16`

The key perf regression to watch is `incremental-typing`, because that is where the visible text path used to dominate full-frame cost.
