# Port optional file watcher

Created: 2026-05-17
Completed: 2026-05-22
Model: GPT-5 Codex

## 背景

original は file watcher による debounced auto-sync を持つ。Rust `cgz` でも opt-in watcher があると local-first index freshness を保ちやすいが、default behavior を勝手に変えるべきではない。

## 期待する状態

- user が明示したときだけ watcher を起動できる
- file changes は debounce され、incremental sync に渡される
- `.codegraph`、build outputs、config excludes は watch 対象から外れる

## 実装メモ

- Reference original files: `src/sync/watcher.ts`, `src/sync/index.ts`
- Rust implementation area: `crates/codegraph/src/main.rs`, `crates/codegraph/src/lib.rs`, `crates/codegraph/src/config.rs`
- incremental sync issue 完了後に実装する

## 検証

- Watcher unit tests where possible
- Manual smoke test with temporary project
- `cargo test --all --all-features`

## 解決方法

Implemented opt-in file watcher as `cgz watch` subcommand with the following changes:

### Files changed

1. **`crates/codegraph/Cargo.toml`** — Added `notify = "6"` dependency (cross-platform file notification library).

2. **`crates/codegraph/src/watcher.rs`** (new) — Core watcher module with:
   - `WatcherConfig` struct with configurable `debounce_ms` (default 300ms)
   - `run_watcher()` function that starts a `notify::RecommendedWatcher` on the project root
   - Debounces file change events by collecting paths during the debounce window, then running `CodeGraph::sync()` on timeout
   - `should_watch_path()` — extends `should_include_file()` by also excluding `.codegraph/` directory components
   - `is_relevant_event()` — filters for Create/Modify/Remove events, skipping Access/Any/Other
   - 5 unit tests covering path filtering, config consistency, and debounce defaults

3. **`crates/codegraph/src/lib.rs`** — Added `pub mod watcher;` and `CodeGraph::config()` accessor (needed by watcher to read config without consuming the struct).

4. **`crates/codegraph/src/main.rs`** — Added `Watch { path, debounce }` subcommand to the CLI, invoking `run_watcher()`.

5. **`crates/codegraph/tests/watcher_smoke.rs`** (new) — 6 integration tests:
   - Codegraph dir exclusion
   - Build output exclusion (target/, build/, dist/)
   - Source file inclusion (rs, ts, mbt)
   - Consistency with `should_include_file`
   - `WatcherConfig` default values
   - End-to-end sync respects config (only indexes source files)

All 55 tests pass (`cargo test --all --all-features`).
