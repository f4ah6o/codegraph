# Port CSharp extraction

Created: 2026-05-17
Completed: 2026-05-22
Model: GPT-5 Codex

## 背景

original は C# の class、method、using、attribute、inheritance 抽出を持つ。ASP.NET route detection と impact analysis の前提として Rust 側にも C# extractor が必要である。

## 期待する状態

- classes、methods、interfaces、usings を抽出できる
- attributes と inheritance/implements 相当の edges または unresolved refs を記録できる
- ASP.NET route resolver が利用できる metadata がある

## 実装メモ

- Reference original files: `src/extraction/languages/csharp.ts`, `__tests__/extraction.test.ts`
- Rust implementation area: `crates/codegraph/src/extraction*`, `crates/codegraph/tests/`
- route extraction は framework resolver issue で扱う

## 検証

- C# fixture tests
- `cargo test --all --all-features`

## 解決方法

- C# 専用 extractor を registry に追加し、generic extraction から分離した。
- class、interface、struct、enum、method、property、using を抽出し、async/static/visibility metadata を保持した。
- attribute、base class、interface を unresolved refs として記録し、ASP.NET route resolver が参照できる attribute metadata を残した。
- C# fixture tests を追加し、`cargo test --all --all-features` で確認した。
