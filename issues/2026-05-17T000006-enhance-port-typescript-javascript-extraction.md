# Port TypeScript and JavaScript extraction

Created: 2026-05-17
Model: GPT-5 Codex

## 背景

original は TypeScript/JavaScript/TSX/JSX の symbol、export、import、component 抽出を持つ。Rust `cgz` は generic regex に留まるため、実用的な JS/TS project の context quality が不足する。

## 期待する状態

- functions、classes、interfaces、type aliases、exported consts、arrow functions を抽出できる
- default/named/namespace/side-effect/type imports を unresolved refs として記録できる
- TSX/JSX component を `component` または適切な node kind として扱える

## 実装メモ

- Reference original files: `src/extraction/languages/typescript.ts`, `src/extraction/languages/javascript.ts`, `__tests__/extraction.test.ts`
- Rust implementation area: `crates/codegraph/src/extraction*`, `crates/codegraph/tests/`
- Tree-sitter を優先し、fallback regex は限定的にする

## 検証

- TypeScript/JavaScript fixture tests
- `cargo test --all --all-features`

