# Port web framework route resolvers

Created: 2026-05-17
Completed: 2026-05-22
Model: GPT-5 Codex

## 背景

original は Django、Flask、FastAPI、Express、Laravel、Rails、Spring、Go routers、ASP.NET、Vapor、React Router、SvelteKit などの route detection を持つ。Rust `cgz` でも route node と handler refs を作ることで web app の context quality が上がる。

## 期待する状態

- framework route files/decorators/builders から `route` nodes を作れる
- route nodes から handler symbol へ `references` edge を作れる
- framework detection は project config または deterministic file patterns で行う

## 実装メモ

- Reference original files: `src/resolution/frameworks/**`, `__tests__/frameworks*.test.ts`
- Rust implementation area: `crates/codegraph/src/extraction*`, `crates/codegraph/src/graph.rs`, `crates/codegraph/tests/`
- MoonBit Sol route extraction は既存実装を壊さない

## 検証

- Framework route fixture tests
- `cargo test --all --all-features`

## 解決方法

- TypeScript/JavaScript 抽出で Express 形式の `app/router/server.METHOD(path, ..., handler)` と React Router の `<Route path=...>` から `route` node を作成するようにした。
- Python 抽出で Flask/FastAPI 形式の `@app.get(...)` などの route decorator から `route` node を作成し、handler 関数への `references` edge 用 unresolved reference を追加した。
- Next.js App Router API route と `pages/`、SvelteKit/Vue style の file-based page route を deterministic file pattern から route node 化した。
- 既存の MoonBit Sol route 抽出と Python decorator reference の互換性を保った。
- `crates/codegraph/tests/original_fixture_parity.rs` に framework route と file-based route の fixture tests を追加し、`cargo test --all --all-features` で確認した。
