# Improve graph query depth and paths

Created: 2026-05-17
Model: GPT-5 Codex

## 背景

original の graph traversal は callers、callees、impact radius、dependency chains などを task context に活用する。Rust `cgz` の graph query を強化して、より深い関係を deterministic に返せるようにする必要がある。

## 期待する状態

- callers/callees/impact が depth、duplicate suppression、ordering を一貫して扱う
- path/dependency chain を返す API または report がある
- CLI/MCP 出力が large graph でも bounded で読みやすい

## 実装メモ

- Reference original files: `src/graph/traversal.ts`, `src/graph/queries.ts`, `src/context/index.ts`
- Rust implementation area: `crates/codegraph/src/graph.rs`, `crates/codegraph/src/lib.rs`, `crates/codegraph/src/mcp.rs`
- output limit と deterministic sort を明確にする

## 検証

- Graph traversal fixture tests
- `cargo test --all --all-features`

