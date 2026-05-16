# Add first-class agent workflow guidance for cgz

Created: 2026-05-16
Completed: 2026-05-16
Model: GPT-5 Codex

## 背景

`cgz` は Rust CLI として `init`, `status`, `query`, `context`, `affected`,
`serve --mcp` を持っているが、agent が日常の開発ワークフローでどう使うべきかの
指針がまだ薄い。

特に moonrepo では `cgz` を codegraph repository として管理するのではなく、
インストール済み CLI として使いたい。agent が勝手に `.codegraph/` を作ったり、
古い index を根拠に判断したりしないよう、明確な作法が必要。

## 期待する状態

- agent 向けに、最初に `cgz status <path>` で index 状態を確認する方針が書かれている
- 未初期化の場合は `cgz init -i <path>` を明示的な操作として扱い、自動実行しない
- read-only な探索では `cgz files`, `cgz query`, `cgz context`, `cgz affected` を使う
- `cgz` の結果は探索補助であり、最終確認は対象 repo の通常 test/check で行う、と明記されている
- Codex / Claude / 他 agent が参照できる短い workflow document または installer 出力がある

## 例

```bash
command -v cgz
cgz status .
cgz context --path . "調べたいタスク"
git diff --name-only | xargs cgz affected --path .
```

## 補足

moonrepo 側には `cgz-workflow` skill と read-only `just cgz-*` helper を追加する。
この issue は、同じ思想を codegraph 側の README、installer、または agent 向け
ドキュメントへ反映するための追跡用。

## 解決方法

`docs/AGENT_WORKFLOW.md` を追加し、agent が最初に `cgz status <path>` で
index 状態を確認すること、未初期化時の `cgz init -i <path>` は明示的な
workspace-changing 操作として扱うこと、read-only 探索には `cgz files`,
`cgz query`, `cgz context`, `cgz affected` を使うことを明記した。

また README からこの workflow へリンクし、MCP server instructions にも
CodeGraph の結果は探索補助であり最終確認は対象 repository の通常 test/check
で行う、という境界を追加した。
