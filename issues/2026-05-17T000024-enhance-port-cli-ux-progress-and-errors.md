# Port CLI UX progress and error reporting

Created: 2026-05-17
Model: GPT-5 Codex

## 背景

original CLI は progress、duration/count formatting、parse/read error summary、misleading error avoidance などの UX を持つ。Rust `cgz` も large project indexing で利用者が状況を把握できる表示が必要である。

## 期待する状態

- index/sync の progress と summary が読みやすい
- parse/read/unsupported/lock などの error が分類される
- duration と counts が human-readable で表示される
- `--quiet` と JSON 出力は machine-readable のまま保たれる

## 実装メモ

- Reference original files: `src/bin/codegraph.ts`, `src/ui/shimmer-progress.ts`
- Rust implementation area: `crates/codegraph/src/main.rs`, `crates/codegraph/src/types.rs`, `crates/codegraph/src/lib.rs`
- animation は必須ではなく、まず deterministic な progress/error reporting を優先する

## 検証

- CLI output snapshot or assertion tests
- `cargo test --all --all-features`

