# Update AGENTS.md for Rust cgz

Created: 2026-05-17
Model: GPT-5 Codex

## 背景

`AGENTS.md` は Rust `cgz` 向けの内容を含むが、project guidance として original TypeScript CodeGraph の前提が混ざりやすい。今後 original からの port 作業を進めるため、`main` は Rust `cgz` が canonical であること、original は reference branch であることをより明確にする必要がある。

## 期待する状態

- `AGENTS.md` が Rust workspace、crate、CLI、library API、issue workflow を中心に説明している
- `original-codegraph/main` は port 元 reference としてのみ扱うことが明記されている
- build/test/release/publish 禁止事項が現在の Rust crate 運用に合っている
- issue 作成・完了・reopen・pending の手順が曖昧でない

## 実装メモ

- Reference original files: `CLAUDE.md`, `IMPLEMENTATION_PLAN.md`
- Rust implementation area: `AGENTS.md`, `CLAUDE.md`, `README.md`
- original 向けの npm / Node / TypeScript release 手順は AGENTS から除外する
- `cgz` の command examples と repository branch policy を source of truth として残す

## 検証

- `rg -n "npm|TypeScript|original-codegraph|cargo|cgz" AGENTS.md`
- `cargo test --all --all-features`

