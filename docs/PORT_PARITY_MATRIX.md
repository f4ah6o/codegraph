# Original-to-cgz Port Parity Matrix

This matrix tracks intentional ports from the TypeScript reference on
`original-codegraph/main` to the canonical Rust `cgz` implementation on `main`.
The original branch is reference material only; do not merge it into `main`.

## Status Legend

| Status | Meaning |
| --- | --- |
| Done | Equivalent Rust behavior exists and is covered by tests or docs. |
| Partial | Rust has related behavior, but known original behavior remains unported. |
| Planned | A scoped issue exists and no implementation should be assumed complete. |
| Not porting | The behavior is intentionally omitted; the reason is recorded. |

## Matrix

| Area | Original reference | Rust target | Status | Related issue | Notes |
| --- | --- | --- | --- | --- | --- |
| Port roadmap | `original-codegraph/main` | `issues/`, `AGENTS.md` | Done | [2026-05-17T000001](../issues/closed/2026-05-17T000001-spec-roadmap-port-original-codegraph-to-cgz.md) | Rust `cgz` is canonical; original is a reference branch. |
| Project instructions | `CLAUDE.md`, `README.md` | `AGENTS.md`, `CLAUDE.md`, `README.md` | Done | [2026-05-17T000002](../issues/closed/2026-05-17T000002-docs-update-agents-for-rust-cgz.md) | Repository guidance now names Rust commands and branch policy. |
| Port parity matrix | `src/bin/codegraph.ts`, `src/mcp/tools.ts`, `src/extraction/**`, `src/resolution/**`, `src/sync/**`, `src/installer/**`, `__tests__/**` | `docs/PORT_PARITY_MATRIX.md`, `issues/` | Done | [2026-05-17T000003](../issues/closed/2026-05-17T000003-spec-define-port-parity-matrix.md) | This document is the inventory and status surface for follow-up work. |
| Original fixture parity harness | `__tests__/extraction.test.ts`, `__tests__/resolution.test.ts`, `__tests__/frameworks*.test.ts` | `crates/codegraph/tests/` | Done | [2026-05-17T000004](../issues/closed/2026-05-17T000004-enhance-add-original-fixture-test-harness.md) | Reusable Rust fixture helpers cover source extraction and temp project indexing without the TypeScript runner. |
| Extractor registry structure | `src/extraction/languages/index.ts`, `src/extraction/tree-sitter-types.ts` | `crates/codegraph/src/extraction.rs` or `crates/codegraph/src/extraction/**` | Done | [2026-05-17T000005](../issues/closed/2026-05-17T000005-enhance-improve-language-extractor-registry.md) | Rust now has a named extractor registry dispatching Rust, MoonBit, and generic extractors without changing extraction behavior. |
| TypeScript and JavaScript extraction | `src/extraction/languages/typescript.ts`, `src/extraction/languages/javascript.ts`, `__tests__/extraction.test.ts` | `crates/codegraph/src/extraction*`, `crates/codegraph/tests/` | Done | [2026-05-17T000006](../issues/closed/2026-05-17T000006-enhance-port-typescript-javascript-extraction.md) | Dedicated TS/JS extraction covers functions, classes, interfaces, type aliases, exported arrow functions, imports, and TSX/JSX component nodes. |
| Python extraction | `src/extraction/languages/python.ts`, `__tests__/extraction.test.ts` | `crates/codegraph/src/extraction*`, `crates/codegraph/tests/` | Done | [2026-05-17T000007](../issues/closed/2026-05-17T000007-enhance-port-python-extraction.md) | Dedicated Python extraction covers functions, classes, methods, import references, and decorator metadata for route resolver follow-up work. |
| Go extraction | `src/extraction/languages/go.ts`, `__tests__/extraction.test.ts` | `crates/codegraph/src/extraction*`, `crates/codegraph/tests/` | Done | [2026-05-17T000008](../issues/closed/2026-05-17T000008-enhance-port-go-extraction.md) | Dedicated Go extraction covers package modules, structs, interfaces, functions, receiver methods, grouped imports, and call references. |
| Java and Kotlin extraction | `src/extraction/languages/java.ts`, `src/extraction/languages/kotlin.ts`, `__tests__/extraction.test.ts` | `crates/codegraph/src/extraction*`, `crates/codegraph/tests/` | Planned | [2026-05-17T000009](../issues/2026-05-17T000009-enhance-port-java-kotlin-extraction.md) | Spring metadata is tracked through the framework resolver issue. |
| C# extraction | `src/extraction/languages/csharp.ts`, `__tests__/extraction.test.ts` | `crates/codegraph/src/extraction*`, `crates/codegraph/tests/` | Planned | [2026-05-17T000010](../issues/2026-05-17T000010-enhance-port-csharp-extraction.md) | ASP.NET route metadata depends on extractor support. |
| PHP and Ruby extraction | `src/extraction/languages/php.ts`, `src/extraction/languages/ruby.ts`, `__tests__/extraction.test.ts` | `crates/codegraph/src/extraction*`, `crates/codegraph/tests/` | Planned | [2026-05-17T000011](../issues/2026-05-17T000011-enhance-port-php-ruby-extraction.md) | Laravel and Rails routes are tracked separately. |
| Swift extraction | `src/extraction/languages/swift.ts`, `__tests__/extraction.test.ts` | `crates/codegraph/src/extraction*`, `crates/codegraph/tests/` | Planned | [2026-05-17T000012](../issues/2026-05-17T000012-enhance-port-swift-extraction.md) | Vapor and SwiftUI metadata remains planned. |
| Dart, Pascal, and Scala extraction | `src/extraction/languages/dart.ts`, `src/extraction/languages/pascal.ts`, `src/extraction/languages/scala.ts` | `crates/codegraph/src/extraction*`, `crates/codegraph/tests/` | Planned | [2026-05-17T000013](../issues/2026-05-17T000013-enhance-port-dart-pascal-scala-extraction.md) | Parser dependency decisions may move portions to pending. |
| Liquid, Vue, and Svelte extraction | `src/extraction/liquid-extractor.ts`, `src/extraction/vue-extractor.ts`, `src/extraction/svelte-extractor.ts` | `crates/codegraph/src/extraction*`, `crates/codegraph/tests/` | Planned | [2026-05-17T000014](../issues/2026-05-17T000014-enhance-port-liquid-vue-svelte-extraction.md) | Component and template relationship extraction remains unported. |
| Core import resolution | `src/resolution/import-resolver.ts`, `src/resolution/name-matcher.ts`, `src/resolution/path-aliases.ts`, `src/resolution/index.ts` | `crates/codegraph/src/db.rs`, `crates/codegraph/src/graph.rs`, `crates/codegraph/src/extraction*` | Partial | [2026-05-17T000015](../issues/2026-05-17T000015-enhance-port-import-resolution-core.md) | Rust has name-oriented graph behavior; deterministic path and alias resolution is planned. |
| Web framework route resolvers | `src/resolution/frameworks/**`, `__tests__/frameworks*.test.ts` | `crates/codegraph/src/extraction*`, `crates/codegraph/src/graph.rs`, `crates/codegraph/tests/` | Partial | [2026-05-17T000016](../issues/2026-05-17T000016-enhance-port-framework-route-resolvers-web.md) | Existing MoonBit Sol route support should be preserved while web routes are added. |
| Graph query depth and paths | `src/graph/traversal.ts`, `src/graph/queries.ts`, `src/context/index.ts` | `crates/codegraph/src/graph.rs`, `crates/codegraph/src/lib.rs`, `crates/codegraph/src/mcp.rs` | Partial | [2026-05-17T000017](../issues/2026-05-17T000017-enhance-port-graph-query-depth-and-paths.md) | Current context and affected reports exist; deeper traversal and path reporting are planned. |
| Indexed files tree output | `src/mcp/tools.ts`, `src/bin/codegraph.ts` | `crates/codegraph/src/main.rs`, `crates/codegraph/src/mcp.rs`, `crates/codegraph/src/db.rs` | Planned | [2026-05-17T000018](../issues/2026-05-17T000018-enhance-port-codegraph-files-tree-output.md) | Rust `cgz files` currently focuses on language counts. |
| Explore/context output | `src/mcp/tools.ts`, `src/context/index.ts`, `src/context/formatter.ts` | `crates/codegraph/src/lib.rs`, `crates/codegraph/src/mcp.rs`, `crates/codegraph/src/graph.rs` | Partial | [2026-05-17T000019](../issues/2026-05-17T000019-enhance-port-codegraph-explore-output.md) | Current context output lacks original-style source sections and relationship maps. |
| MCP tool polish | `src/mcp/tools.ts`, `src/mcp/server-instructions.ts`, `src/mcp/transport.ts` | `crates/codegraph/src/mcp.rs`, `crates/codegraph/tests/mcp_smoke.rs` | Partial | [2026-05-17T000020](../issues/2026-05-17T000020-enhance-port-mcp-tool-polish.md) | Rust MCP exists; schema polish and error behavior remain planned. |
| Incremental sync | `src/sync/index.ts`, `src/extraction/index.ts` | `crates/codegraph/src/lib.rs`, `crates/codegraph/src/db.rs`, `crates/codegraph/src/main.rs` | Planned | [2026-05-17T000021](../issues/2026-05-17T000021-enhance-port-incremental-sync.md) | Rust `sync` still rebuilds broadly. |
| Optional file watcher | `src/sync/watcher.ts`, `src/sync/index.ts` | `crates/codegraph/src/main.rs`, `crates/codegraph/src/lib.rs`, `crates/codegraph/src/config.rs` | Planned | [2026-05-17T000022](../issues/2026-05-17T000022-enhance-port-file-watcher.md) | Depends on incremental sync; default behavior should remain explicit. |
| Installer and Claude config workflow | `src/installer/index.ts`, `src/installer/config-writer.ts`, `src/installer/claude-md-template.ts` | `crates/codegraph/src/main.rs`, installer module | Planned | [2026-05-17T000023](../issues/2026-05-17T000023-enhance-port-installer-claude-config.md) | Config writes require careful non-destructive behavior and user confirmation. |
| CLI UX progress and errors | `src/bin/codegraph.ts`, `src/ui/shimmer-progress.ts` | `crates/codegraph/src/main.rs`, `crates/codegraph/src/types.rs`, `crates/codegraph/src/lib.rs` | Partial | [2026-05-17T000024](../issues/2026-05-17T000024-enhance-port-cli-ux-progress-and-errors.md) | Deterministic progress and error summaries should come before animation. |
| Evaluation benchmarks | `__tests__/evaluation/**`, `docs/SEARCH_QUALITY_LOOP.md`, `README.md` | `crates/codegraph/tests/`, `docs/` | Partial | [2026-05-17T000025](../issues/2026-05-17T000025-enhance-port-evaluation-benchmarks.md) | `agent_context_eval` exists; original-style scoring and fixture coverage remain planned. |
| Node version checks | `src/bin/node-version-check.ts`, `__tests__/node-version-check.test.ts` | None | Not porting | None | Rust binary distribution does not require Node runtime validation. |
| TypeScript package publishing script | `publish.js`, `package.json`, `package-lock.json` | `Cargo.toml`, `crates/codegraph/Cargo.toml`, `justfile`, `CHANGELOG.md` | Not porting | None | Rust release flow uses Cargo and repository release helpers instead of npm publishing. |
| TypeScript-specific tree-sitter patch script | `scripts/patch-tree-sitter-dart.js` | None | Not porting | None | Rust parser dependency handling should be captured in language-specific issues if needed. |
| TypeScript runtime debug scripts | `debug_python_ast.js`, `debug_python_ast2.js`, `test_python_inheritance.js` | None | Not porting | None | One-off debug scripts are not product behavior; reproduce useful cases as Rust fixtures instead. |

## Maintenance Rules

- Add a row when a new original behavior area is discovered.
- Link each planned or partial row to an issue unless the row is explicitly
  `Not porting`.
- When closing a related issue, update the row status and notes in this matrix.
- Use `original-codegraph/main:<path>` as the conceptual source reference when
  checking original files locally with `git show` or `git ls-tree`.
