# Port CLI UX progress and error reporting

Created: 2026-05-17
Completed: 2026-05-23
Model: GPT-5 Codex

## 背景

original CLI は progress、duration/count formatting、parse/read error summary、misleading error avoidance などの UX を持つ。Rust `cgz` も large project indexing で利用者が状況を把握できる表示が必要である。

## 期待する状態

- index/sync の progress と summary が読みやすい
- parse/read/unsupported/lock などの error が分類される
- duration と counts が human-readable で表示される
- `--quiet` と JSON 出力は machine-readable のまま保たれる

## 実装メモ

- Reference original files: `src/bin/codegraph.ts`, `src/ui/shimmer-progress.ts`
- Rust implementation area: `crates/codegraph/src/main.rs`, `crates/codegraph/src/types.rs`, `crates/codegraph/src/lib.rs`
- animation は必須ではなく、まず deterministic な progress/error reporting を優先する

## 検証

- CLI output snapshot or assertion tests
- `cargo test --all --all-features`

## 解決方法

- `IndexErrorCategory` と構造化された `IndexError` を追加し、read/parse/unsupported/lock の error category を CLI 出力に表示するようにした。
- `index`/`sync`/`init -i` で deterministic な progress 開始メッセージ、human-readable な count/duration summary、index failure 時の non-zero exit を実装した。
- Rust parse error と unsupported file の CLI integration test、quiet mode と JSON 出力維持の test を追加した。
- `cargo test --all --all-features` と opencode review-loop の exact `LGTM` で確認した。
