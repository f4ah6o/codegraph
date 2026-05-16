# Port web framework route resolvers

Created: 2026-05-17
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

