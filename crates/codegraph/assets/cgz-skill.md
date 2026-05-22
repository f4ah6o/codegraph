# cgz CodeGraph Skill

Use this skill when you need local codebase orientation, symbol search,
task-focused context, or affected-test hints from a project indexed by `cgz`.

## Rules

- Start with `cgz status --path <project>` before relying on graph results.
- Treat `cgz init -i <project>` and `cgz index <project>` as workspace-changing
  actions; run them only when the user asks or approves indexing.
- Use `cgz sync --path <project>` after local edits when an existing index is
  present and you need current graph results.
- Treat all `cgz` output as navigation evidence, not proof. Finish with the
  repository's normal tests, type checks, linters, or build commands.
- Inspect source files before editing even when `cgz context` points to likely
  locations.

## Core Workflow

```bash
command -v cgz
cgz status --path <project>
cgz files --path <project> --format tree
cgz query --path <project> <symbol-or-file-term>
cgz context --path <project> "<task, symbol, or feature>"
cgz affected --path <project> <changed-file>...
```

Use JSON when you need structured evidence:

```bash
cgz status --path <project> --json
cgz query --path <project> <term> --json
cgz context --path <project> "<task>" --json
cgz affected --path <project> <changed-file>... --json
```

## Command Guide

- `cgz status --path <project>` checks whether the graph exists and reports
  file, node, edge, stale-file, and timestamp metadata.
- `cgz files --path <project> --format tree` gives a compact indexed file map.
- `cgz files --path <project> --format flat --pattern "*.rs"` filters indexed
  files by glob.
- `cgz query --path <project> <term>` finds matching symbols and files.
- `cgz context --path <project> "<task>"` builds a task-oriented context pack
  from matching graph nodes and source snippets.
- `cgz affected --path <project> <files...>` suggests tests related to changed
  files using imports, package heuristics, and language-specific test naming.
- `cgz callers --path <project> <symbol>` and `cgz callees --path <project>
  <symbol>` explore call relationships.
- `cgz impact --path <project> <symbol>` expands likely dependents.
- `cgz paths --path <project> <from> <to>` searches graph paths between
  symbols.
- `cgz serve --mcp --path <project>` exposes CodeGraph over MCP.

## Practical Patterns

Find the implementation for a task:

```bash
cgz context --path <project> "change authentication token refresh behavior"
```

Plan verification from current Git changes:

```bash
git diff --name-only | xargs cgz affected --path <project>
```

Get machine-readable affected-test reasons:

```bash
git diff --name-only | xargs cgz affected --path <project> --json
```

Refresh an existing graph after edits:

```bash
cgz sync --path <project>
cgz status --path <project> --json
```

## MCP Mapping

When using the MCP server, start with `codegraph_status`, then use
`codegraph_files`, `codegraph_search`, `codegraph_context`,
`codegraph_affected`, `codegraph_callers`, `codegraph_callees`,
`codegraph_impact`, `codegraph_node`, and `codegraph_explore`.

Pass `projectPath` explicitly when querying a project outside the MCP server's
startup path.
