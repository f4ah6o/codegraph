# Port core import resolution

Created: 2026-05-17
Model: GPT-5 Codex

## 背景

original は import resolver、name matcher、path aliases を持ち、unresolved refs を project 内 symbols/files に接続する。Rust `cgz` は name-based resolution が中心で、cross-file accuracy を上げる余地がある。

## 期待する状態

- relative imports、package/module imports、language-specific path conventions を deterministic に解決できる
- name matching は ambiguity を扱い、誤った edge を作りにくい
- resolution 結果は callers/callees/impact/context に反映される

## 実装メモ

- Reference original files: `src/resolution/import-resolver.ts`, `src/resolution/name-matcher.ts`, `src/resolution/path-aliases.ts`, `src/resolution/index.ts`
- Rust implementation area: `crates/codegraph/src/db.rs`, `crates/codegraph/src/graph.rs`, `crates/codegraph/src/extraction*`
- framework-specific resolution は別 issue で扱う

## 検証

- Cross-file import fixture tests
- `cargo test --all --all-features`

