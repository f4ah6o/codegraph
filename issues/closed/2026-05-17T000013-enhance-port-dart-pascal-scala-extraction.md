# Port Dart Pascal and Scala extraction

Created: 2026-05-17
Completed: 2026-05-22
Model: GPT-5 Codex

## 背景

original は Dart、Pascal/Delphi、Scala の extractors を持つ。Rust `cgz` の supported language list と実際の extraction quality を近づけるため、残りの language coverage を task として分ける。

## 期待する状態

- Dart class/function/enum/mixin/extension/import を抽出できる
- Pascal/Delphi の unit/class/procedure/function/form 関連情報を抽出できる
- Scala class/object/trait/function/import を抽出できる

## 実装メモ

- Reference original files: `src/extraction/languages/dart.ts`, `src/extraction/languages/pascal.ts`, `src/extraction/languages/scala.ts`
- Rust implementation area: `crates/codegraph/src/extraction*`, `crates/codegraph/tests/`
- 依存 parser が難しい場合は pending issue へ移動する

## 検証

- Dart/Pascal/Scala fixture tests
- `cargo test --all --all-features`

## 解決方法

- Dart/Pascal/Scala 専用 extractor を registry に追加し、generic extraction から分離した。
- Dart の class、function、enum、mixin、extension、type alias、import、call refs を抽出した。
- Pascal/Delphi の unit、uses、class、procedure、function、class method、inheritance、call refs を抽出した。
- Scala の class、object、trait、function/method、type alias、import、extends/with refs を抽出した。
- Dart/Pascal/Scala fixture tests を追加し、`cargo test --all --all-features` で確認した。
