# AGENTS.md

This file provides guidance to Codex (Codex.ai/code) when working with code in this repository.

## Project Overview

CodeGraph is a local-first code intelligence system that builds a semantic knowledge graph from any codebase. It provides structural understanding of code relationships using tree-sitter for AST parsing and SQLite for storage.

**Key characteristics:**
- Headless library (no UI) - purely an API
- Node.js runtime (works standalone, in Electron, or any Node environment)
- Per-project data stored in `.codegraph/` directory
- Deterministic extraction from AST, not AI-generated summaries

## Build and Development Commands

```bash
# Build
npm run build          # Compile TypeScript and copy assets

# Test
npm test               # Run all tests once
npm run test:watch     # Run tests in watch mode

# Clean
npm run clean          # Remove dist/ directory
```

## Running a Single Test

```bash
npx vitest run __tests__/extraction.test.ts           # Run specific test file
npx vitest run __tests__/extraction.test.ts -t "TypeScript"  # Run tests matching pattern
```

## Architecture

### Core Module Structure

```
src/
├── index.ts              # Main CodeGraph class - public API entry point
├── types.ts              # All TypeScript interfaces and types
├── db/                   # SQLite database layer
│   ├── index.ts          # DatabaseConnection class
│   ├── queries.ts        # QueryBuilder with prepared statements
│   └── schema.sql        # Table definitions with FTS5 search
├── extraction/           # Tree-sitter AST parsing
│   ├── index.ts          # ExtractionOrchestrator
│   ├── tree-sitter.ts    # Universal parser wrapper
│   └── grammars.ts       # Language detection and grammar loading
├── resolution/           # Reference resolver
│   ├── index.ts          # ReferenceResolver orchestrator
│   ├── import-resolver.ts
│   ├── name-matcher.ts
│   └── frameworks/       # Framework-specific patterns (React, Express, Laravel, etc.)
├── graph/                # Graph traversal and queries
│   ├── index.ts          # GraphQueryManager
│   ├── traversal.ts      # GraphTraverser (BFS/DFS, impact radius)
│   └── queries.ts        # High-level graph queries
├── context/              # Context building for AI assistants
│   ├── index.ts          # ContextBuilder
│   └── formatter.ts      # Markdown/JSON output formatting
├── sync/                 # Incremental update system
│   ├── index.ts
│   └── git-hooks.ts      # Post-commit hook management
├── installer/            # Interactive installer
│   ├── index.ts          # Installer orchestrator
│   ├── banner.ts         # ASCII art banner
│   ├── Codex-md-template.ts # AGENTS.md template generator
│   ├── config-writer.ts  # Configuration file writing
│   └── prompts.ts        # User prompts
├── mcp/                  # Model Context Protocol server
│   ├── index.ts          # MCPServer class
│   ├── tools.ts          # MCP tool definitions
│   └── transport.ts      # Stdio transport
└── bin/codegraph.ts      # CLI entry point
```

### Key Classes

- **CodeGraph** (`src/index.ts`): Main entry point. Lifecycle methods (`init`, `open`, `close`), indexing (`indexAll`, `sync`), graph queries (`traverse`, `getCallGraph`, `getImpactRadius`), context building (`buildContext`)

- **ExtractionOrchestrator** (`src/extraction/index.ts`): Coordinates file scanning, parsing, and storing. Uses tree-sitter native bindings for each supported language

- **GraphTraverser** (`src/graph/traversal.ts`): BFS/DFS traversal, call graph construction, impact radius calculation, path finding

- **ReferenceResolver** (`src/resolution/index.ts`): Resolves unresolved references after full indexing using framework patterns, import resolution, and name matching

### Database Schema

SQLite database with:
- `nodes`: Code symbols (functions, classes, methods, etc.)
- `edges`: Relationships (calls, imports, extends, contains, etc.)
- `files`: Tracked source files with content hashes
- `unresolved_refs`: References pending resolution
- `nodes_fts`: FTS5 virtual table for full-text search

### Supported Languages

TypeScript, JavaScript, TSX, JSX, Svelte, Python, Go, Rust, Java, C, C++, C#, PHP, Ruby, Swift, Kotlin, Dart, Liquid, Pascal

### Node and Edge Types

**NodeKind**: `file`, `module`, `class`, `struct`, `interface`, `trait`, `protocol`, `function`, `method`, `property`, `field`, `variable`, `constant`, `enum`, `enum_member`, `type_alias`, `namespace`, `parameter`, `import`, `export`, `route`, `component`

**EdgeKind**: `contains`, `calls`, `imports`, `exports`, `extends`, `implements`, `references`, `type_of`, `returns`, `instantiates`, `overrides`, `decorates`

## CLI Usage

```bash
codegraph init [path]       # Initialize in project
codegraph index [path]      # Full index
codegraph sync [path]       # Incremental update
codegraph status [path]     # Show statistics
codegraph query <search>    # Search symbols
codegraph context <task>    # Build context for AI
codegraph hooks install     # Install git auto-sync
codegraph serve --mcp       # Start MCP server
```

## MCP Tools Best Practices

Use these tools **directly in the main session** for fast code exploration (replaces the need for Explore agents in most cases):

| Tool | Use For |
|------|---------|
| `codegraph_explore` | **Deep exploration** — comprehensive context for a topic in ONE call |
| `codegraph_context` | Quick context for a task (lighter than explore) |
| `codegraph_search` | Find symbols by name (functions, classes, types) |
| `codegraph_callers` | Find what calls a function |
| `codegraph_callees` | Find what a function calls |
| `codegraph_impact` | See what's affected by changing a symbol |
| `codegraph_node` | Get details + source code for a symbol |

### Important
CodeGraph provides **code context**, not product requirements. For new features, still ask the user about:
- UX preferences and behavior
- Edge cases and error handling
- Acceptance criteria

## Releases

Releases are published to npm **and** mirrored as GitHub Releases on the
[Releases page](https://github.com/f4ah6o/codegraph/releases), which is
where most users look for change history. `CHANGELOG.md` at the repo root is
the source of truth — each GitHub Release's notes are extracted from it.

## Pull Requests

Create pull requests against the `origin` repository (`f4ah6o/codegraph`), not
the `upstream` repository (`colbymchenry/codegraph`). If a PR is opened against
upstream by mistake, leave an apology comment, close it, and recreate the PR on
origin.

### Writing changelog entries

When the user asks for a changelog entry for a new version:

1. Add a new `## [X.Y.Z] - YYYY-MM-DD` block at the **top** of `CHANGELOG.md`
   (directly under the intro, above the previous version).
2. Group changes under `### Added`, `### Changed`, `### Fixed`, `### Removed`,
   `### Deprecated`, `### Security` — only include sections that have entries.
3. Write entries from the **user's perspective**, not the implementation's.
   Lead with the observable symptom or capability, then mention internals only
   if a user needs them (e.g., to work around an existing bad install).
4. Add the link reference at the bottom:
   `[X.Y.Z]: https://github.com/f4ah6o/codegraph/releases/tag/vX.Y.Z`

### Release commands (the user runs these)

After the changelog entry is written and the version is bumped in `package.json`:

```bash
git add package.json package-lock.json CHANGELOG.md
git commit -m "release: X.Y.Z (<one-line summary>)"
git push

npm publish

git tag vX.Y.Z
git push origin vX.Y.Z
gh release create vX.Y.Z \
  --title "vX.Y.Z" \
  --notes-file <(awk '/^## \[X.Y.Z\]/,/^## \[/{ if (/^## \[/ && !/X.Y.Z/) exit; print }' CHANGELOG.md)
```

Do **not** run `npm publish`, `git tag`, `git push`, or `gh release create`
yourself — these are publish actions that affect shared state. Write the file,
hand the user the commands.

## Test Structure

Tests are in `__tests__/` directory with files mirroring the module structure:
- `foundation.test.ts` - Database, config, directory management
- `extraction.test.ts` - Tree-sitter parsing for all languages
- `resolution.test.ts` - Reference resolution
- `graph.test.ts` - Traversal and graph queries
- `context.test.ts` - Context building
- `sync.test.ts` - Incremental updates and git hooks

Tests use temporary directories created with `fs.mkdtempSync` and cleaned up after each test.

## issues について

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
