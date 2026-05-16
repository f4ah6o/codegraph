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
