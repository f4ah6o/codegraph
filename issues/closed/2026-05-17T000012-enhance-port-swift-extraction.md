# Port Swift extraction

Created: 2026-05-17
Completed: 2026-05-22
Model: GPT-5 Codex

## 背景

original は Swift の classes、structs、protocols、functions、imports、inheritance/conformance を抽出する。Swift/Vapor/SwiftUI codebase で `cgz` を使うには Rust 側の coverage が必要である。

## 期待する状態

- class、struct、protocol、function、method を抽出できる
- import と protocol conformance / inheritance を refs として記録できる
- Vapor/SwiftUI route/component resolver の前提 metadata を保持できる

## 実装メモ

- Reference original files: `src/extraction/languages/swift.ts`, `__tests__/extraction.test.ts`
- Rust implementation area: `crates/codegraph/src/extraction*`, `crates/codegraph/tests/`
- framework resolver は別 issue で扱う

## 検証

- Swift fixture tests
- `cargo test --all --all-features`

## 解決方法

- Swift 専用 extractor を registry に追加し、generic extraction から分離した。
- class、struct、protocol、enum、typealias、function、method、import を抽出し、visibility/static/async metadata を保持した。
- inheritance / protocol conformance を unresolved refs として記録し、Vapor/SwiftUI resolver が参照できる symbol metadata を残した。
- Swift fixture tests を追加し、`cargo test --all --all-features` で確認した。
