# Port optional file watcher

Created: 2026-05-17
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

