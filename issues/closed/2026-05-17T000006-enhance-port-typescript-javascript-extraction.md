# Port TypeScript and JavaScript extraction

Created: 2026-05-17
Completed: 2026-05-22
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

## 解決方法

`crates/codegraph/src/extraction.rs` に TypeScript / JavaScript 専用 extractor を追加し、
registry から `typescript_javascript` として dispatch するようにした。tree-sitter
parser dependency は追加せず、既存構成に合わせた bounded regex 実装で
function / class / interface / type alias / exported const arrow function を抽出する。

import は default、named、namespace、side-effect、type import、relative import を
module 名の `import` node と `imports` unresolved reference として記録する。
TSX / JSX では uppercase の function / class / arrow function から `component`
node も追加する。

`crates/codegraph/tests/original_fixture_parity.rs` に TypeScript / JavaScript fixture
tests を追加し、exports、imports、arrow functions、JSX component dispatch を検証した。
`docs/PORT_PARITY_MATRIX.md` の該当行も Done に更新した。
