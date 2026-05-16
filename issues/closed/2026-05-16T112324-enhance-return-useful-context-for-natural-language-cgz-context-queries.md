# Return useful context for natural language cgz context queries

Created: 2026-05-16
Completed: 2026-05-16
Model: GPT-5 Codex

## 背景

`repos/calver.mbt` で `cgz init -i` 後に `cgz context` を試したところ、
シンボル名だけの query は有効だったが、自然文の task query はヘッダだけを返した。

```bash
cgz context --path repos/calver.mbt "change parse_with_scheme validation for invalid scheme order"
```

結果:

```text
## Context: change parse_with_scheme validation for invalid scheme order
```

一方で、短いシンボル名を指定すると期待どおりコード付き context が返る。

```bash
cgz context --path repos/calver.mbt parse_with_scheme
cgz context --path repos/calver.mbt scheme
```

## 期待する状態

- 自然文 task query でも、含まれる識別子や重要語から関連シンボルを拾える
- 何も見つからない場合は、空の context ではなく「シンボル名やファイル名で再試行してほしい」などの具体的な案内を出す
- agent が `cgz context` を開発計画の入口として使ったとき、空出力で見落としが起きにくい

## 補足

現状でも `cgz query --path repos/calver.mbt parse_with_scheme --json` は正しく候補を返している。
`context` 側で自然文から検索語を抽出するか、空結果時に `query` への誘導を出すと実用性が上がる。

## 解決方法

`CodeGraph::build_context` で自然文 task をそのまま検索した後、識別子・file path
らしい token・重要語を抽出して追加検索する fallback を追加した。
検索結果は node id で重複排除し、`max_nodes` の範囲で context に含める。

また `search_nodes` の対象に `file_path` を追加し、file 名や path 由来の query でも
候補を返しやすくした。何も見つからない場合は header だけを返さず、具体的な
symbol 名・file 名・package 名・`cgz query --json <term>` での再試行を案内する。

自然文から `parse_with_scheme` を拾えることと、空結果時に案内を返すことを
CLI test で固定した。
