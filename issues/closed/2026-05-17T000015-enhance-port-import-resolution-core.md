# Port core import resolution

Created: 2026-05-17
Completed: 2026-05-22
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

## 解決方法

- unresolved refs の解決を name-only から path-first resolver に拡張した。
- relative imports と language-specific extension/index conventions を indexed file nodes に解決するようにした。
- `tsconfig.json` / `jsconfig.json` の `compilerOptions.paths` を JSONC tolerant に読み取り、alias imports を indexed files に解決するようにした。
- name fallback は同ランク候補が複数ある場合に edge を作らないようにし、曖昧な誤解決を避けるようにした。
- relative import と tsconfig path alias の cross-file fixture tests を追加し、`cargo test --all --all-features` で確認した。
