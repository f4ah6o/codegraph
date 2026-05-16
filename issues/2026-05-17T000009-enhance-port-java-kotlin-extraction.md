# Port Java and Kotlin extraction

Created: 2026-05-17
Model: GPT-5 Codex

## 背景

original は Java/Kotlin の classes、interfaces、methods、imports、annotations、inheritance などを抽出する。Spring や Kotlin service code の context を作るには Rust 側の extractor 強化が必要である。

## 期待する状態

- Java/Kotlin の class、interface、method、function を抽出できる
- imports、extends、implements、Kotlin suspend metadata を記録できる
- annotations を framework route resolver が参照できる

## 実装メモ

- Reference original files: `src/extraction/languages/java.ts`, `src/extraction/languages/kotlin.ts`, `__tests__/extraction.test.ts`
- Rust implementation area: `crates/codegraph/src/extraction*`, `crates/codegraph/tests/`
- Spring resolver は別 issue で扱う

## 検証

- Java/Kotlin fixture tests
- `cargo test --all --all-features`

