# Port installer and Claude config workflow

Created: 2026-05-17
Model: GPT-5 Codex

## 背景

original の installer は MCP server 設定、permissions、CLAUDE.md instructions、local project initialization を支援する。Rust `cgz install` は未実装メッセージのみなので、利用開始までの導線が弱い。

## 期待する状態

- `cgz install` が global/local Claude config target を扱える
- existing user config を破壊せず、CodeGraph block だけを追加・更新する
- auto-allow permissions は user の明示選択で設定する
- local install では `cgz init -i` を実行するか確認する

## 実装メモ

- Reference original files: `src/installer/index.ts`, `src/installer/config-writer.ts`, `src/installer/claude-md-template.ts`
- Rust implementation area: `crates/codegraph/src/main.rs`, new installer module if needed
- publish/install side effects は user confirmation を必須にする

## 検証

- Temp HOME/config fixture tests
- `cargo test --all --all-features`

