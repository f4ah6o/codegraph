# Polish MCP tools toward original behavior

Created: 2026-05-17
Completed: 2026-05-22
Model: GPT-5 Codex

## 背景

Rust `cgz` は MCP server を持つが、original の tool schema、dynamic descriptions、error wording、cross-project behavior と差分がある。AI agent が使いやすい tool surface にするため polish が必要である。

## 期待する状態

- tool schemas が descriptions、defaults、enums、required fields を明確に持つ
- `codegraph_search` の kind filter が機能する
- initialized でない project の error が actionable である
- cross-project queries の挙動と cache policy が明確である

## 実装メモ

- Reference original files: `src/mcp/tools.ts`, `src/mcp/server-instructions.ts`, `src/mcp/transport.ts`
- Rust implementation area: `crates/codegraph/src/mcp.rs`, `crates/codegraph/tests/mcp_smoke.rs`
- MCP protocol compatibility を壊さない

## 検証

- `cargo test -p cgz --test mcp_smoke`
- `cargo test --all --all-features`

## 解決方法

- MCP tool schema に property description、default、enum、min/max bounds を追加し、各 tool の入力契約を明確にした。
- `codegraph_search` の `kind` option を `NodeKind` に parse して `SearchOptions` に渡すようにし、不正な kind は actionable error にした。
- initialized でない project の error に `cgz init --index` と `projectPath` の案内を含め、`projectPath` 指定時も初期化済み project を求める文言にした。
- server instructions と `projectPath` schema description に cross-project query は call 単位で扱い、cross-project result cache を持たないことを明記した。
- MCP smoke test で schema metadata、kind filter、uninitialized project error を検証し、`cargo test --all --all-features` で確認した。
