# Port incremental sync

Created: 2026-05-17
Completed: 2026-05-22
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

## 解決方法

- `CodeGraph::sync` を full reindex から content hash ベースの incremental update に変更し、unchanged file は reindex せず skipped として数えるようにした。
- changed/new file は既存の file/node/edge/ref index を削除してから再抽出・再挿入し、deleted file は DB から file/node/edge/ref を削除するようにした。
- resolver 由来 edge を sync/index 後に再生成するようにし、changed/deleted file による reference graph の整合性を保つようにした。
- `IndexResult` と CLI summary に deleted count を追加した。
- `crates/codegraph/tests/sync_incremental.rs` を追加し、unchanged skip、changed/new indexing、deleted file removal を検証した。
