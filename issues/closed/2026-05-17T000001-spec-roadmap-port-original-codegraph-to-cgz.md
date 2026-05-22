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

## 解決方法

Completed: 2026-05-22

`original-codegraph/main` から Rust `cgz` へ port する対象を timestamp 順の
task issue として登録済みであることを確認した。具体的には docs / parity matrix /
fixture harness / extractor registry / language extractors / import resolution /
framework routes / graph queries / files and explore output / MCP polish /
sync-watch / installer / CLI UX / evaluation benchmarks の各単位が
`issues/2026-05-17T000002-...` から `issues/2026-05-17T000025-...` として
独立して追跡できる。

`AGENTS.md` の branch policy でも `main` を canonical Rust `cgz` branch とし、
`original-codegraph/main` は reference として扱い自動 merge しない方針を明記済みである。

検証として、2026-05-17 作成の issue 一覧と各 issue の `Created: 2026-05-17` /
`Model: GPT-5 Codex` metadata を確認した。
