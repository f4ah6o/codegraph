# cgz

`cgz` is the Rust CLI in this repository. It builds and queries a local
CodeGraph database under `.codegraph/`.

The original upstream README is kept as `README.org.md`.

## Package

| Field | Value |
|---|---|
| Crate | `cgz` |
| Binary | `cgz` |
| Library name | `codegraph` |
| Current crate version | `2026.5.3` |
| License | MIT |
| Repository | `https://github.com/f4ah6o/codegraph` |

The crate lives at `crates/codegraph`. The workspace root contains the Rust
workspace files and the upstream TypeScript implementation.

## What cgz Stores

`cgz init` creates `.codegraph/` in the target project. The database file is
`.codegraph/codegraph.db`.

The default config includes source patterns for common languages plus MoonBit
files:

- `*.mbt`
- `*.mbti`
- `*.mbt.md`
- `moon.mod.json`
- `moon.pkg.json`
- `moon.pkg`

The default excludes include `.git`, `node_modules`, `vendor`, `dist`, `build`,
`out`, `target`, `.codegraph`, `.moon`, and `.mooncakes`.

## Build

```bash
cargo build -p cgz
```

Release build:

```bash
cargo build --release -p cgz
```

## Test

```bash
cargo test --all --all-features
```

The `oracle_parity` test target requires the `oracle-tests` feature:

```bash
cargo test -p cgz --features oracle-tests --test oracle_parity
```

## Commands

```bash
cgz init [path]        # create .codegraph/
cgz init -i [path]     # create .codegraph/ and index
cgz uninit --force     # remove .codegraph/
cgz index [path]       # rebuild the index
cgz sync [path]        # sync changed files
cgz status [path]      # print file, node, edge, and DB size stats
cgz status --json      # print status as JSON
cgz query <search>     # search indexed nodes
cgz query <search> -j  # print search results as JSON
cgz files              # print indexed file counts by language
cgz context <task>     # print context for a task
cgz context <task> -j  # print context evidence as JSON
cgz affected <files>   # print affected test files from import dependents
cgz affected <files> -j # print affected test evidence as JSON
cgz serve --mcp        # start the MCP server
cgz unlock [path]      # remove .codegraph/codegraph.lock
```

## Agent Workflow

Agents should start with `cgz status <path>` before relying on graph results.
If a project is not initialized, `cgz init -i <path>` is a deliberate
workspace-changing operation, not a read-only discovery step.

For day-to-day read-only exploration, use `cgz files`, `cgz query`,
`cgz context`, and `cgz affected`. Treat their output as navigation context and
finish with the target repository's normal tests, type checks, or build checks.

See [docs/AGENT_WORKFLOW.md](./docs/AGENT_WORKFLOW.md) for the short playbook.

Running `cgz` with no subcommand currently prints:

```text
Rust CodeGraph installer is not implemented yet. Run `cgz init -i` in a project.
```

## Release Helpers

The root `justfile` defines:

```bash
just release-check
just publish-cli
just release-tag
just release
```

`release-check` runs Cargo tests, builds the release binary, and runs
`cargo publish --dry-run` for `crates/codegraph/Cargo.toml`.

## Issue Management

Issues are managed locally as markdown files in the `issues/` directory, rather
than on GitHub Issues.

- Open issues live in `issues/`
- Closed issues are moved to `issues/closed/`
- Pending or blocked issues are moved to `issues/pending/`

File naming convention: `{YYYY-MM-DDThhmmss}-{category}-{slug}.md`

For the full issue workflow, see [AGENTS.md](./AGENTS.md).

This approach is inspired by [shiguredo/http3-rs](https://github.com/shiguredo/http3-rs/blob/develop/AGENTS.md).
