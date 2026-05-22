# Improve language extractor registry

Created: 2026-05-17
Completed: 2026-05-22
Model: GPT-5 Codex

## 背景

現在の Rust extraction は `extraction.rs` に集約されており、original のように language ごとの extractor を増やすほど保守性が落ちる。今後の細かい port を安全に進めるため、dispatch と helper の構造を整理する必要がある。

## 期待する状態

- language ごとの extractor 追加が局所的な変更で済む
- 共通 helper と language-specific logic の境界が明確である
- 既存 Rust/MoonBit の抽出結果が変わらない

## 実装メモ

- Reference original files: `src/extraction/languages/index.ts`, `src/extraction/tree-sitter-types.ts`
- Rust implementation area: `crates/codegraph/src/extraction.rs` または `crates/codegraph/src/extraction/**`
- 大規模な挙動追加ではなく、後続 port の受け皿を作る

## 検証

- `cargo test --all --all-features`
- 既存 MoonBit/Rust extraction tests が変更前と同じ期待値を満たす

## 解決方法

`crates/codegraph/src/extraction.rs` に `LanguageExtractor` registry を追加し、
language から named extractor (`rust`, `moonbit`, `generic`) を引ける dispatch に
整理した。既存の Rust / MoonBit / generic extractor 本体はそのまま利用し、
後続の language-specific port では registry に entry を追加するだけで dispatch
を拡張できる形にした。

`registered_extractor_name` を追加し、test harness から Rust、MoonBit、TypeScript
の dispatch を検証できるようにした。TypeScript は現時点では generic extractor
に送られ、詳細な TypeScript/JavaScript port は別 issue の対象として残している。

`docs/PORT_PARITY_MATRIX.md` の extractor registry 行も Done に更新した。
