# Headless Edit Baseline

- Date: 2026-03-13
- Commit: `b0a1a8e90a0c640ecd4053df8deb90094adb11f8` plus local edits
- Command: `cargo bench --bench headless -- --noplot 'playground/headless-script/<scenario>'`

## Current Medians

- `large-paste`: `241.93 ms`
- `incremental-typing`: `323.35 ms`
- `undo-redo-burst`: `179.18 ms`
- `backspace-burst`: `675.29 ms`
- `delete-forward-burst`: `674.65 ms`

## Baseline Comparison

Baseline used for comparison: clean `HEAD` at `b0a1a8e90a0c640ecd4053df8deb90094adb11f8`

- `large-paste`: `250.95 ms` -> `241.93 ms` (`-3.6%`)
- `incremental-typing`: `1.7675 s` -> `323.35 ms` (`-81.7%`)
- `undo-redo-burst`: `848.16 ms` -> `179.18 ms` (`-78.9%`)
- `backspace-burst`: `3.4081 s` -> `675.29 ms` (`-80.2%`)
- `delete-forward-burst`: `3.4678 s` -> `674.65 ms` (`-80.5%`)
