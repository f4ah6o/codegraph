# Improve graph query depth and paths

Created: 2026-05-17
Completed: 2026-05-22
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

## 解決方法

- callers/callees traversal を depth 付きの deterministic BFS にし、同一 node の重複出力を抑制した。
- impact radius の edge 重複を抑制し、出力順を deterministic にした。
- `GraphPath` と `CodeGraph::find_paths` を追加し、bounded depth / bounded count で symbol 間の dependency/call path を返せるようにした。
- CLI に `callers`、`callees`、`impact`、`paths` を追加し、depth / limit / JSON 出力を扱えるようにした。
- MCP の callers/callees/impact に depth・limit を反映し、新しい `codegraph_paths` tool を追加した。
- `crates/codegraph/tests/graph_traversal.rs` に depth、duplicate suppression、path search の fixture test を追加し、`cargo test --all --all-features` で確認した。
