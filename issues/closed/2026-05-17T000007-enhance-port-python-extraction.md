# Port Python extraction

Created: 2026-05-17
Completed: 2026-05-22
Model: GPT-5 Codex

## 背景

original は Python の function、class、import 抽出と framework route detection の前提を持つ。Rust `cgz` の generic extraction では Python codebase の search/context が弱い。

## 期待する状態

- function definitions、class definitions、methods を抽出できる
- simple/from/aliased/relative/wildcard imports を unresolved refs として記録できる
- decorator 情報を framework route resolver が利用できる形で保持できる

## 実装メモ

- Reference original files: `src/extraction/languages/python.ts`, `__tests__/extraction.test.ts`
- Rust implementation area: `crates/codegraph/src/extraction*`, `crates/codegraph/tests/`
- Django/Flask/FastAPI routes は別 issue で扱う

## 検証

- Python fixture tests
- `cargo test --all --all-features`

## 解決方法

- Python 専用 extractor を registry に追加し、generic extraction から分離した。
- function/class/method、async method、`@staticmethod`、decorator metadata を抽出し、decorator は `Decorates` unresolved refs と signature に保持した。
- simple/from/aliased/relative/wildcard import を import node と `Imports` unresolved refs として記録した。
- original fixture parity harness に Python fixture を追加し、`cargo test --all --all-features` で確認した。
