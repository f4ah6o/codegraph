# Define original-to-cgz port parity matrix

Created: 2026-05-17
Model: GPT-5 Codex

## 背景

original TypeScript 実装と Rust `cgz` の機能差分が大きく、どの挙動が port 済み・未対応・不要なのかを一目で確認できない。細かい task issue を進める前に parity matrix が必要である。

## 期待する状態

- CLI、MCP、extractor、framework routes、resolution、sync、installer、docs、tests の差分が表で分かる
- 各行に original reference path、Rust target、status、関連 issue がある
- 「port しない」判断も理由付きで記録できる

## 実装メモ

- Reference original files: `src/bin/codegraph.ts`, `src/mcp/tools.ts`, `src/extraction/**`, `src/resolution/**`, `src/sync/**`, `src/installer/**`, `__tests__/**`
- Rust implementation area: `docs/`, `issues/`
- matrix は docs に置き、issue から参照する

## 検証

- matrix 内の related issue links が存在する
- `rg -n "original-codegraph/main|port|parity" docs issues`

