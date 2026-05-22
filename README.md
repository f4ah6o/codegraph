# cgz

[![Crates.io](https://img.shields.io/crates/v/cgz.svg)](https://crates.io/crates/cgz)
[![Docs.rs](https://docs.rs/cgz/badge.svg)](https://docs.rs/cgz)
[![License](https://img.shields.io/crates/l/cgz.svg)](https://github.com/f4ah6o/codegraph/blob/main/LICENSE)
[![Publish crate](https://github.com/f4ah6o/codegraph/actions/workflows/publish-crate.yml/badge.svg)](https://github.com/f4ah6o/codegraph/actions/workflows/publish-crate.yml)

`cgz` is the Rust CLI in this repository. It builds and queries a local
CodeGraph database under `.codegraph/`.

## Package

| Field | Value |
|---|---|
| Crate | `cgz` |
| Binary | `cgz` |
| Library name | `codegraph` |
| Current crate version | `2026.5.4` |
| License | MIT |
| Repository | `https://github.com/f4ah6o/codegraph` |

The crate lives at `crates/codegraph`. The workspace root is now a Rust Cargo
workspace; the original TypeScript CodeGraph code is tracked separately on the
`original-codegraph/main` branch.

## Upstream Acknowledgement

`cgz` is a Rust rework of the original
[CodeGraph](https://github.com/colbymchenry/codegraph) project. This repository
keeps the upstream project's ideas and history in view while evolving the tool
as a local-first Rust CLI and library.

## Supported Languages and Frameworks

`cgz` detects and indexes source files for these languages and file formats:

| Area | Support |
|---|---|
| TypeScript / JavaScript | `.ts`, `.tsx`, `.js`, `.jsx`, `.mjs`, `.cjs` |
| Python | `.py`, `.pyw` |
| Go | `.go` |
| Rust | `.rs` |
| Java / Kotlin | `.java`, `.kt`, `.kts` |
| C / C++ | `.c`, `.h`, `.cpp`, `.cc`, `.cxx`, `.hpp`, `.hxx` |
| C# | `.cs` |
| PHP / Ruby | `.php`, `.rb`, `.rake` |
| Swift | `.swift` |
| Dart | `.dart` |
| Pascal | `.pas`, `.dpr`, `.dpk`, `.lpr`, `.dfm`, `.fmx` |
| Scala | `.scala`, `.sc` |
| MoonBit | `.mbt`, `.mbti`, `.mbt.md`, `moon.mod.json`, `moon.pkg.json`, `moon.pkg` |
| Liquid / Vue / Svelte | `.liquid`, `.vue`, `.svelte` |

Framework-aware indexing currently includes MoonBit Sol route extraction,
Liquid render/include/section references, and Vue/Svelte component and template
references. Web framework route resolvers from the upstream TypeScript project
are being ported intentionally; see
[docs/PORT_PARITY_MATRIX.md](./docs/PORT_PARITY_MATRIX.md) for current parity.

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
See [docs/AGENT_CONTEXT_EVAL.md](./docs/AGENT_CONTEXT_EVAL.md) for the
automated context-success fixture used to guard agent navigation quality.

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
