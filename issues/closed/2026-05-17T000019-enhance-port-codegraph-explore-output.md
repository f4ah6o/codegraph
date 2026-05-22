# Port codegraph explore output

Created: 2026-05-17
Completed: 2026-05-22
Model: GPT-5 Codex

## 背景

original の `codegraph_explore` は deep exploration 用に source sections、relationship map、project-size budget guidance、output truncation を持つ。Rust `cgz` の explore/context はまだ単純で、agent が一回で理解できる情報量の調整が必要である。

## 期待する状態

- relevant files ごとに source sections を grouping して返せる
- relationship map と additional relevant files を含められる
- project size に応じた explore call budget を MCP description に反映できる
- output は bounded で truncation 表示がある

## 実装メモ

- Reference original files: `src/mcp/tools.ts`, `src/context/index.ts`, `src/context/formatter.ts`
- Rust implementation area: `crates/codegraph/src/lib.rs`, `crates/codegraph/src/mcp.rs`, `crates/codegraph/src/graph.rs`
- natural language query は search/context で扱い、explore は symbol/file terms を優先する設計を検討する

## 検証

- Context/explore fixture tests
- MCP smoke test
- `cargo test --all --all-features`

## 解決方法

- `CodeGraph::build_explore_report` と explore report 型を追加し、query に関連する source sections を file ごとに grouping して返せるようにした。
- call/reference relationship から relationship map と additional relevant files を生成し、source file 数、section size、relationship 数を bounded に扱うようにした。
- project file count に応じた explore budget guidance と truncation 表示を report/MCP 出力に含めた。
- MCP `codegraph_explore` を新しい explore report formatter に切り替え、tool description に project size 別の呼び出し guidance と maxFiles bounds を反映した。
- context/explore fixture test と MCP smoke test を更新し、`cargo test --all --all-features` で確認した。
