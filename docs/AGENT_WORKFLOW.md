# Agent Workflow for cgz

`cgz` is a local code exploration aid. It can speed up codebase orientation,
but it does not replace the target repository's normal tests, type checks, or
manual source review.

## Start With Status

Before using CodeGraph in a repository, check whether an index exists and how
fresh it appears:

```bash
command -v cgz
cgz status <path>
```

If the project is not initialized, treat initialization as an explicit
workspace-changing action:

```bash
cgz init -i <path>
```

Do not run initialization automatically during read-only exploration. It creates
`.codegraph/` in the target project and indexes local files.

## Read-Only Exploration

After confirming that the project is initialized, prefer read-only commands for
agent planning and code navigation:

```bash
cgz files --path <path>
cgz query --path <path> <symbol-or-file-term>
cgz context --path <path> "<task or symbol>"
cgz context --path <path> "<task or symbol>" --json
cgz affected --path <path> <changed-file>...
cgz affected --path <path> <changed-file>... --json
```

For changed files from Git:

```bash
git diff --name-only | xargs cgz affected --path <path>
```

`cgz context` and `cgz query` work best with concrete symbol names, file names,
package names, and short domain terms. Natural-language task descriptions are
accepted, but agents should still inspect the returned files before editing.
Use `cgz context --json` when a machine-readable list of matched terms, files,
symbols, and match reasons is more useful than markdown context.
For MCP clients, call `codegraph_context` with `format: "json"` for the same
structured report.
Use `cgz affected --json` when planning verification; its `debug[].matchedBy`
field separates direct test inputs, import-dependent tests, MoonBit
same-package tests, and Rust name-heuristic test matches.
For MCP clients, call `codegraph_affected` with a `files` array to get the same
affected-test report.

## Verification Boundary

CodeGraph results are structural context, not proof of correctness. Before
claiming a change is complete, run the target repository's usual verification
commands such as tests, type checks, linters, or build commands.

If results look stale or incomplete, run `cgz status <path>` again and ask for an
explicit indexing step before relying on the graph.
