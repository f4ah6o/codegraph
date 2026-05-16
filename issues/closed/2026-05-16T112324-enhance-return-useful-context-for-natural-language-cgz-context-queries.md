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

## Reopened: 2026-05-16

`moonbitlang/core` のような大きめの MoonBit codebase で、自然文内の型名を拾えない
ケースが残っていた。

再現手順:

```bash
git clone --depth 1 https://github.com/moonbitlang/core.git /tmp/moonbitlang-core
cargo build -p cgz
target/debug/cgz init /tmp/moonbitlang-core
target/debug/cgz index /tmp/moonbitlang-core
target/debug/cgz context "How is Json implemented?" --path /tmp/moonbitlang-core
```

実際の結果:

```text
## Context: How is Json implemented?

No matching symbols or files were found.
Try a concrete symbol name, file name, package/module name, or a shorter code term. For candidate discovery, run `cgz query --json <term>`.
```

一方で、同じ index に対して短い symbol query は有効:

```bash
target/debug/cgz context Json --path /tmp/moonbitlang-core
```

この場合は `builtin/json.mbt` の `pub enum Json` と `json/json.mbt` の
`Json::as_*` / `Json::stringify` などが返る。

前回の修正で空出力は改善されたが、自然文から `Json` のような短い CamelCase
識別子を候補語として使えていない。自然文 task query の入口としては、ユーザーが
型名や API 名を文章に含めた場合に該当 symbol へ到達できる必要がある。

## 解決方法

`context_search_terms` の重要語判定で、3 文字以上かつ ASCII uppercase を含む token
を候補に含めるようにした。これにより `How is Json implemented?` のような自然文から
短い CamelCase 型名 `Json` を抽出できる。

MoonBit fixture に `pub enum Json` を含む再現テストを追加し、`cgz context "How is Json implemented?"`
が `json.mbt` と `enum Json` を返すことを確認した。

## Reopened: 2026-05-16

`d80a59c` の修正後に `moonbitlang/core` の実 index で再確認したところ、空結果ではなくなったが
`Json` ではなく文頭の `How` が検索語として使われ、`Show` / `output` の context が先に返る。

再現手順:

```bash
git clone --depth 1 https://github.com/moonbitlang/core.git /tmp/moonbitlang-core
cargo build -p cgz
target/debug/cgz init /tmp/moonbitlang-core
target/debug/cgz index /tmp/moonbitlang-core
target/debug/cgz context "How is Json implemented?" --path /tmp/moonbitlang-core
```

実際の結果は `builtin/traits.mbt` の `trait Show` と多数の `output` method が先頭に並び、
`Json` の実装 context が返らない。単体で確認すると `cgz query How --path /tmp/moonbitlang-core`
が `Show` に fuzzy match している。

原因候補:

`is_useful_context_term` が `term.chars().any(|c| c.is_ascii_uppercase())` だけで短い token を
採用するため、文頭で大文字になった普通の英単語 `How` も短い CamelCase 型名と同じ扱いになる。

期待する状態:

自然文 query では文頭 capitalization の英単語を重要語にしない。`Json` のような短い型名を拾う場合も、
少なくとも stop word / question word の除外、または `Uppercase + lowercase...` だけではなく既存 symbol
への exact match を優先するなど、普通の英文 token が fuzzy match で関連 context を押し出さないようにする。

## 解決方法

自然文 query の token 抽出で、`how`, `what`, `why`, `who`, `which`, `is`,
`implemented` などの疑問文・説明文由来の stop word を除外した。これにより文頭
capitalization の `How` が短い型名候補として扱われず、`Json` が優先して検索される。

回帰テストでは `Show` / `output` を含む fixture に対して
`cgz context "How is Json implemented?" --json` を実行し、`search_terms` に `How`
が含まれず `Json` が含まれること、先頭 match が `json.mbt` の `Json` になることを固定した。
