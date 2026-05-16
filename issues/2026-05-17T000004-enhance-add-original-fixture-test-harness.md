# Add original fixture parity test harness

Created: 2026-05-17
Model: GPT-5 Codex

## 背景

original の `__tests__` には extractor、resolution、framework、MCP の期待挙動が多く含まれる。Rust port の regressions を防ぐには、TypeScript runtime に依存しない Rust fixture harness が必要である。

## 期待する状態

- Rust tests で fixture source から nodes、edges、imports、routes を検証できる
- original の代表的な test cases を Rust fixture として段階的に移植できる
- fixture helper は language-specific test から再利用できる

## 実装メモ

- Reference original files: `__tests__/extraction.test.ts`, `__tests__/resolution.test.ts`, `__tests__/frameworks*.test.ts`
- Rust implementation area: `crates/codegraph/tests/`
- TypeScript test runner は使わず、Rust test helper と tempdir indexing で検証する

## 検証

- `cargo test -p cgz --test original_fixture_parity`
- `cargo test --all --all-features`

