# Port codegraph files tree output

Created: 2026-05-17
Completed: 2026-05-22
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

## 解決方法

- `CodeGraph::list_files` と file list report 型を追加し、`tree`、`flat`、`grouped` format を共有 API として扱えるようにした。
- path filter、pattern filter、metadata 表示、max depth を bounded に処理するようにした。
- `cgz files` に `--format`、`--filter-path`、`--pattern`、`--include-metadata`、`--max-depth` を追加し、新しい option を指定した JSON 出力は report 形式にした。既存の `cgz files --json` は language counts のまま維持した。
- MCP `codegraph_files` で同じ format/filter/metadata/depth option を利用し、format は `tree`、`flat`、`grouped` に限定して曖昧な指定を error にした。
- `crates/codegraph/tests/files_output.rs` と MCP smoke test を追加・更新し、`cargo test --all --all-features` で確認した。
