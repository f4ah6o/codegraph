# Port original CodeGraph behavior to Rust cgz

Created: 2026-05-17
Model: GPT-5 Codex

## 背景

`original-codegraph/main` には TypeScript 実装として installer、MCP tool、framework resolver、多言語 extractor、sync/watch などが存在する。`main` は Rust `cgz` が canonical なので、branch merge ではなく、有用な挙動を Rust に意図的に port する必要がある。

## 期待する状態

- port 対象を task 単位の issue として追跡できる
- 各 task は Rust `cgz` の実装・検証単位として独立している
- TypeScript 実装は reference として扱い、`main` へ自動 merge しない方針が明確である

## 実装メモ

- Reference original files: `src/**`, `__tests__/**`, `IMPLEMENTATION_PLAN.md`
- Rust implementation area: `crates/codegraph/**`, `docs/**`, `AGENTS.md`
- port は user-visible value を優先し、TypeScript の内部構造をそのまま再現しない
- issue は番号代わりに timestamp の昇順で実施する

## 検証

- `ls issues/2026-05-17T*.md | sort`
- `rg -n "Created: 2026-05-17|Model: GPT-5 Codex" issues/2026-05-17T*.md`

