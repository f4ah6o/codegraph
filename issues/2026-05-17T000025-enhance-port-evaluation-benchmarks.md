# Port evaluation benchmarks

Created: 2026-05-17
Model: GPT-5 Codex

## 背景

original は CodeGraph が agent exploration を改善することを evaluation と benchmark で確認している。Rust `cgz` でも extraction/context quality の改善を定量的に追える軽量 evaluation が必要である。

## 期待する状態

- controlled fixtures に対して context が必要 symbol/file を含むか検証できる
- extraction coverage と context quality の regressions を CI で検出できる
- marketing claim ではなく、開発用の engineering metric として扱う

## 実装メモ

- Reference original files: `__tests__/evaluation/**`, `docs/SEARCH_QUALITY_LOOP.md`, `README.md`
- Rust implementation area: `crates/codegraph/tests/`, `docs/`
- large external repositories に依存する test は標準 test suite に入れない

## 検証

- Evaluation fixture tests
- `cargo test --all --all-features`

