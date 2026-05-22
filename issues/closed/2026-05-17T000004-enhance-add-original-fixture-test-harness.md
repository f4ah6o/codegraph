# Add original fixture parity test harness

Created: 2026-05-17
Completed: 2026-05-22
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

## 解決方法

`crates/codegraph/tests/support/mod.rs` に original parity 用の reusable fixture
helper を追加した。`OriginalSourceFixture` は TypeScript runtime を使わず Rust の
`detect_language` / `extract_from_source` で nodes、edges、imports などを検証でき、
`OriginalFixtureProject` は tempdir project を作成して `CodeGraph::index_all` で
index 後の search / route assertions を書ける。

`crates/codegraph/tests/original_fixture_parity.rs` を追加し、original の
`__tests__/extraction.test.ts` と `__tests__/frameworks*.test.ts` から段階的に
移植できる形で、TypeScript symbol/import extraction、Rust project indexing、
MoonBit route fixture の代表ケースを検証した。

`docs/PORT_PARITY_MATRIX.md` の fixture parity harness 行も Done に更新した。
