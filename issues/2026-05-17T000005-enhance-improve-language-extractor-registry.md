# Improve language extractor registry

Created: 2026-05-17
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

