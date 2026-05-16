# AGENTS.md

This file provides guidance to Codex when working in this repository.

## Project Overview

`cgz` is a Rust CLI and library for local-first code intelligence. It builds a
semantic CodeGraph database under `.codegraph/` using tree-sitter parsing and
SQLite storage.

Key characteristics:
- Rust workspace with crate package `cgz`
- CLI binary name: `cgz`
- Library crate name: `codegraph`
- Per-project data stored in `.codegraph/`
- Deterministic extraction from source structure, not AI-generated summaries
- Upstream TypeScript CodeGraph is tracked separately on `original-codegraph/main`

## Build and Development Commands

```bash
cargo build -p cgz
cargo build --release -p cgz
cargo test --all --all-features
cargo publish --dry-run --locked --manifest-path crates/codegraph/Cargo.toml
```

The root `justfile` provides release helpers:

```bash
just release-check
just publish-cli
just release-tag
just release
```

Do not run `cargo publish`, `git tag`, `git push`, or GitHub release commands
unless the user explicitly asks for publish actions.

## Architecture

```
crates/codegraph/
├── Cargo.toml
├── src/
│   ├── main.rs        # CLI entry point
│   ├── lib.rs         # Library API and orchestration
│   ├── config.rs      # Project config and defaults
│   ├── db.rs          # SQLite schema and query layer
│   ├── extraction.rs  # Source scanning and tree-sitter extraction
│   ├── graph.rs       # Search, context, and graph queries
│   ├── mcp.rs         # MCP server/tool surface
│   └── types.rs       # Shared data types
└── tests/
    ├── mcp_smoke.rs
    ├── moonbit_extraction.rs
    ├── moonbit_routes.rs
    └── agent_context_eval.rs
```

## CLI Usage

```bash
cgz init [path]          # create .codegraph/
cgz init -i [path]       # create .codegraph/ and index
cgz uninit --force       # remove .codegraph/
cgz index [path]         # rebuild the index
cgz sync [path]          # sync changed files
cgz status [path]        # print stats
cgz status --json        # print stats as JSON
cgz query <search>       # search indexed nodes
cgz query <search> -j    # print search results as JSON
cgz files                # print indexed file counts by language
cgz context <task>       # print task-oriented context
cgz context <task> -j    # print context evidence as JSON
cgz affected <files>     # print affected test files
cgz affected <files> -j  # print affected test evidence as JSON
cgz serve --mcp          # start the MCP server
cgz unlock [path]        # remove .codegraph/codegraph.lock
```

## Branch Policy

- `main` is the canonical Rust `cgz` branch.
- `original-codegraph/main` tracks the upstream TypeScript CodeGraph project.
- Do not merge `original-codegraph/main` into `main` automatically. Port useful
  upstream behavior intentionally into Rust.
- Existing `origin/upstream/main` is historical and should not be rewritten.

## Releases

Releases are published as the Rust crate `cgz` and mirrored as GitHub Releases
on `f4ah6o/codegraph`. `CHANGELOG.md` is the source of truth for release notes.

When adding a changelog entry for a new version:

1. Add a new `## [X.Y.Z] - YYYY-MM-DD` block at the top of `CHANGELOG.md`.
2. Include only the Keep a Changelog sections that have entries.
3. Write entries from the user's perspective.
4. Add the link reference at the bottom:
   `[X.Y.Z]: https://github.com/f4ah6o/codegraph/releases/tag/vX.Y.Z`

Release commands are run by the user after review:

```bash
git add Cargo.toml Cargo.lock crates/codegraph/Cargo.toml CHANGELOG.md
git commit -m "release: X.Y.Z (<one-line summary>)"
git push

cargo publish --locked --manifest-path crates/codegraph/Cargo.toml

git tag vX.Y.Z
git push origin vX.Y.Z
gh release create vX.Y.Z \
  --title "vX.Y.Z" \
  --notes-file <(awk '/^## \[X.Y.Z\]/,/^## \[/{ if (/^## \[/ && !/X.Y.Z/) exit; print }' CHANGELOG.md)
```

## Issues

- 番号が小さい issues から順に対応すること
- `{YYYY-MM-DDThhmmss}-{category}-{short-description}.md` という命名規則を守ること
  - 日付時刻は issue 作成時の ISO 8601 形式（ファイルシステム安全のためコロンなし）
  - 例: `2026-05-05T150150-spec-stabilize-contract.md`
  - 例: `2026-05-06T091500-bug-fix-parse-error.md`
- 仕様的に対応が難しい場合は issues/pending/ へ移動すること
- issue を作成したらコミットすること
- 1 issue 完了ごとに 1 コミットすること
- Issue の作成日はファイルのタイトルの後に `Created: YYYY-MM-DD` として記載すること
- Issue の完了日はファイルのタイトルの後に `Completed: YYYY-MM-DD` として記載すること
- Issue を作成した LLM の Model と Version をファイルのタイトルの後に `Model: <model-name> <version>` として記載すること
- Issue はなぜこの対応が必要なのかの根拠を明確にすること

### issue が実は解決してなかった場合

- reopen の理由を issue に書いて issues/closed から issues/ に移動すること (git mv を使うこと)
- reopen の理由は、何がどう解決していなかったのかを明確にすること

### バグが見つかった場合

- issues/ 以下にバグを markdown 形式で登録すること
- バグは再現手順を明確にすること
- できる限りの情報を issue 本文に残すこと

### バグを修正した場合

- issues/ 以下のバグを修正した場合は、修正内容を markdown 形式で記載すること
- issues/closed に移動すること (git mv を使うこと)
- issues/closed に移動するときは issue ファイルに「## 解決方法」セクションを追記し、何をどう修正したかを明記すること

### 設計判断が必要な issue の場合

- 外部依存の追加や設計判断が必要で保留中の issue は `issues/pending/` に置くこと
- issues/pending に移動するときは issue ファイルに pending にした理由を明記すること
- pending の issue は修正せずそのまま残す（close しない）

### issue workflow の参考

この issue 管理方式は [shiguredo/http3-rs](https://github.com/shiguredo/http3-rs/blob/develop/AGENTS.md) の AGENTS.md を参考にしている。
ただし本プロジェクトでは連番 (`issues/SEQUENCE`) の代わりに日付時刻をファイル名に使用する。
