# Rust affected analysis misses Tokio spawn tests
Created: 2026-05-16
Model: GPT-5 Codex

## 問題

`tokio-rs/tokio` を CodeGraph で indexing した後、中心的な runtime API である `tokio/src/task/spawn.rs` を変更ファイルとして `codegraph affected` に渡しても affected tests が 0 件になる。

## 再現手順

1. `git clone --depth 1 https://github.com/tokio-rs/tokio.git /tmp/tokio-rs-tokio`
2. Node.js 22 で CodeGraph CLI をビルド済みの `dist/bin/codegraph.js` から実行する。
3. `node dist/bin/codegraph.js init /tmp/tokio-rs-tokio`
4. `/tmp/tokio-rs-tokio` で `node /Users/fu2hito/src/codegraph/dist/bin/codegraph.js index --verbose`
5. `/tmp/tokio-rs-tokio` で `node /Users/fu2hito/src/codegraph/dist/bin/codegraph.js affected tokio/src/task/spawn.rs --json`

## 実際の結果

```json
{
  "changedFiles": [
    "tokio/src/task/spawn.rs"
  ],
  "affectedTests": [],
  "totalDependentsTraversed": 0
}
```

## 期待する結果

`tokio::spawn` は Tokio の公開 API として広く使われているため、少なくとも `tokio/tests/task_spawn.rs` など spawn 関連の test file が候補に含まれるべき。

## 根拠

同じ index に対して `codegraph query spawn` は `tokio/src/task/builder.rs`、scheduler、join set など複数の Rust symbol を検出できている。一方で `affected` は file dependent traversal に依存しており、Rust の `mod` 宣言、`pub use`、crate-root re-export、public API 経由の test 参照が file dependency edge として十分に解決されていない可能性がある。

## 補足

`codegraph affected tokio/tests/task_spawn.rs --json` は変更ファイル自体を test として返すため、test file 判定は動作している。問題は source file から dependent test file への到達にある。

## 解決方法

Rust source file 変更時に、file stem と test file 名を照合する name heuristic を
`build_affected_report` に追加した。たとえば `tokio/src/task/spawn.rs` の stem
`spawn` は `tokio/tests/task_spawn.rs` に含まれるため、import dependent edge が
未解決でも affected test 候補に含める。

`debug[].matchedBy.rustNameHeuristic` にこの経路で一致した test を出すようにし、
direct test input / import dependents / MoonBit same-package と区別できるようにした。
`tokio/src/task/spawn.rs` と `tokio/tests/task_spawn.rs` の fixture で回帰テストを追加した。
