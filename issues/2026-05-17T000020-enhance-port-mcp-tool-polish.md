# Polish MCP tools toward original behavior

Created: 2026-05-17
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

