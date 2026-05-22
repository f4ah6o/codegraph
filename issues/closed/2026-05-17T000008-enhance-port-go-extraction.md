# Port Go extraction

Created: 2026-05-17
Completed: 2026-05-22
Model: GPT-5 Codex

## 背景

original は Go の function、method、import 抽出を持つ。Rust `cgz` で Go project の impact/context を有効にするには、receiver method と grouped imports の対応が必要である。

## 期待する状態

- top-level functions と receiver methods を区別して抽出できる
- grouped/single/aliased/dot/blank imports を記録できる
- call references が basic graph traversal に接続できる

## 実装メモ

- Reference original files: `src/extraction/languages/go.ts`, `__tests__/extraction.test.ts`
- Rust implementation area: `crates/codegraph/src/extraction*`, `crates/codegraph/tests/`
- Go framework route resolver は別 issue で扱う

## 検証

- Go fixture tests
- `cargo test --all --all-features`

## 解決方法

- Go 専用 extractor を registry に追加し、generic extraction から分離した。
- package、struct、interface、top-level function、receiver method を抽出し、receiver method は `Receiver.Method` の qualified name として保持した。
- single/grouped/aliased/dot/blank import を import node と `Imports` unresolved refs として記録した。
- Go call reference fixture を追加し、`cargo test --all --all-features` で確認した。
