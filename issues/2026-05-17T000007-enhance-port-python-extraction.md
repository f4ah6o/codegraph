# Port Python extraction

Created: 2026-05-17
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

