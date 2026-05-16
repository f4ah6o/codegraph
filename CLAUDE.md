# CLAUDE.md

This repository is the Rust `cgz` project.

For current repository instructions, use [AGENTS.md](./AGENTS.md). The key
points are:

- `main` is the canonical Rust `cgz` branch.
- Build with `cargo build -p cgz`.
- Test with `cargo test --all --all-features`.
- The crate lives in `crates/codegraph`.
- The original TypeScript CodeGraph project is tracked separately on
  `original-codegraph/main`.
- Do not merge upstream TypeScript changes into `main` automatically; port
  behavior intentionally into Rust.
