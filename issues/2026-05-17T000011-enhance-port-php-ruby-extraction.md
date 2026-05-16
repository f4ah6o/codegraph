# Port PHP and Ruby extraction

Created: 2026-05-17
Model: GPT-5 Codex

## 背景

original は PHP と Ruby の classes/modules、methods/functions、imports/requires、inheritance を抽出する。Laravel/Rails route resolution の基盤として Rust `cgz` にも必要である。

## 期待する状態

- PHP classes、functions、methods、use statements、extends/implements を抽出できる
- Ruby modules、classes、methods、require/require_relative を抽出できる
- framework route resolver が handler symbol と接続できる

## 実装メモ

- Reference original files: `src/extraction/languages/php.ts`, `src/extraction/languages/ruby.ts`, `__tests__/extraction.test.ts`
- Rust implementation area: `crates/codegraph/src/extraction*`, `crates/codegraph/tests/`
- Laravel/Rails specific patterns は別 issue で扱う

## 検証

- PHP/Ruby fixture tests
- `cargo test --all --all-features`

