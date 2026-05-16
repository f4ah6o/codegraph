# Port incremental sync

Created: 2026-05-17
Model: GPT-5 Codex

## 背景

Rust `cgz sync` は現在 `index_all` と同等で、project size が大きいと無駄が大きい。original は content hashing と changed file detection による incremental update の構想・実装を持つ。

## 期待する状態

- unchanged files は reindex しない
- changed files は nodes/edges/refs を置き換える
- deleted files は DB から削除する
- sync result に indexed/skipped/deleted/error counts が出る

## 実装メモ

- Reference original files: `src/sync/index.ts`, `src/extraction/index.ts`
- Rust implementation area: `crates/codegraph/src/lib.rs`, `crates/codegraph/src/db.rs`, `crates/codegraph/src/main.rs`
- DB transaction と per-file delete/insert の整合性を重視する

## 検証

- Sync fixture tests for unchanged/changed/deleted files
- `cargo test --all --all-features`

