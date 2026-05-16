# Port codegraph files tree output

Created: 2026-05-17
Model: GPT-5 Codex

## 背景

original の `codegraph_files` は indexed files を tree/flat/grouped format、path/pattern filter、metadata、max depth 付きで返す。Rust `cgz files` は language counts が中心で、project structure exploration には不足している。

## 期待する状態

- CLI と MCP で indexed file tree を取得できる
- `tree`、`flat`、`grouped` format を support する
- path、pattern、includeMetadata、maxDepth を bounded に扱う

## 実装メモ

- Reference original files: `src/mcp/tools.ts`, `src/bin/codegraph.ts`
- Rust implementation area: `crates/codegraph/src/main.rs`, `crates/codegraph/src/mcp.rs`, `crates/codegraph/src/db.rs`
- 既存 `cgz files --json` の互換性を壊す場合は changelog 対象にする

## 検証

- CLI files output tests
- MCP `codegraph_files` smoke test
- `cargo test --all --all-features`

