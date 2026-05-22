# Improve MoonBit affected test detection from import dependents

Created: 2026-05-16
Completed: 2026-05-16
Model: GPT-5 Codex

## 背景

`repos/calver.mbt` で `cgz init -i` 後に `cgz affected` を試したところ、
主要 source file を渡しても affected test が空になった。

```bash
cgz affected --path repos/calver.mbt --json src/parse.mbt src/scheme.mbt src/semver.mbt
```

結果:

```json
{
  "affectedTests": [],
  "changedFiles": [
    "src/parse.mbt",
    "src/scheme.mbt",
    "src/semver.mbt"
  ]
}
```

同じ repo には `src/semver_test.mbt`, `src/semver_edge_test.mbt`,
`src/oracle_test.mbt`, `src/increment_test.mbt` などが存在する。

## 期待する状態

- MoonBit package 内の source file 変更から、同一 package または import 関係上の test file を候補として返す
- 依存 graph で判断できない場合でも、同一 package の `*_test.mbt` を conservative fallback として返す選択肢を検討する
- `affectedTests` が空の場合は、依存辺が無いのか、test file が未検出なのかを説明できる debug 出力がある

## 補足

`cgz query` と `cgz context` のシンボル検索は `calver.mbt` で有効だった。
影響範囲推定だけが空になっており、MoonBit import/package graph の解釈が不足している可能性がある。

## 解決方法

`cgz affected` の既存 import-dependent 判定に加えて、MoonBit source file 変更時は
同一 package の test file を conservative fallback として返すようにした。
package は indexed files 内の最寄り `moon.pkg.json` / `moon.pkg` で判定し、test file
として `*_test.mbt`, `*_wbtest.mbt`, `*.mbt.md` を扱う。

JSON 出力には `debug` を追加し、各 changed file が direct test input だったのか、
import-dependent または MoonBit same-package fallback で test を拾ったのか、
あるいは候補が無かったのかを確認できるようにした。

同一 package の MoonBit tests が返ること、別 package の tests は混ざらないこと、
直接 test file を渡した場合はその file が保持されることを CLI test で固定した。

## Reopened: 2026-05-16

別の MoonBit repo として `Lampese/NocturneJS` で確認したところ、package-local test が
少ない repo では import-dependent integration test を拾えず、affected tests が空になる。

再現手順:

```bash
git clone --depth 1 https://github.com/Lampese/NocturneJS.git /tmp/moonbit-nocturnejs
cargo build -p cgz
target/debug/cgz init /tmp/moonbit-nocturnejs
target/debug/cgz index /tmp/moonbit-nocturnejs
target/debug/cgz affected runtime/runtime_state.mbt --path /tmp/moonbit-nocturnejs --json
target/debug/cgz affected engine/parser.mbt --path /tmp/moonbit-nocturnejs --json
```

index 結果:

```text
Indexed 331 files, 8971 nodes, 33863 edges in 133586ms
```

実際の結果:

`runtime/runtime_state.mbt` と `engine/parser.mbt` はどちらも affected tests が 0 件で
warning になる。

```json
{
  "affectedTests": [],
  "warnings": [
    "runtime/runtime_state.mbt: no import-dependent tests, MoonBit same-package tests, Rust name-heuristic tests, or Rust workspace tests found"
  ]
}
```

期待する結果:

root package の `nocturnejs_test.mbt` は `@nocturnejs.Engine::new()` と
`engine.eval(...)` を多数実行している。root `moon.pkg.json` は
`Lampese/nocturnejs/runtime` と `Lampese/nocturnejs/value` を import しており、
root API は `@runtime.Vm` を介して parser/runtime に依存する。少なくとも
`runtime/runtime_state.mbt` の変更では `nocturnejs_test.mbt` が候補に含まれるべき。

補足:

この repo は package-local の `*_test.mbt` が root の `nocturnejs_test.mbt` だけなので、
同一 package fallback だけでは主要 package の変更から test に到達できない。
MoonBit の `moon.pkg.json` import graph と root integration test の関係を affected
analysis に反映する必要がある。

性能面の観察:

`.mbt` は 314 files 程度だが index に 133 秒かかり、処理中に `.codegraph/codegraph.db-wal`
が一時的に約 650MB まで増えた。完了後の DB は約 19MB まで戻ったが、MoonBit の解決処理で
大量の中間書き込みが発生している可能性がある。

## 解決方法

Completed: 2026-05-22

MoonBit の package import graph を `cgz affected` 側で構築し、変更 file が属する
package を import する local package を推移的に辿って、その dependent package 内の
test file を affected test として返すようにした。

package 名は root `moon.mod.json` の `name` と package directory から導出し、
`moon.pkg.json` / `moon.pkg` の `import` / `imports` は array / object / string を扱う。
JSON debug には `matchedBy.moonbitPackageDependents` を追加し、この経路で拾った
integration test を確認できるようにした。

root package が `runtime` を import し、`runtime` が `engine` を import する fixture を追加し、
`runtime/runtime_state.mbt` と `engine/parser.mbt` のどちらの変更でも root の
`app_test.mbt` が返ること、無関係 package の test が混ざらないことを固定した。
