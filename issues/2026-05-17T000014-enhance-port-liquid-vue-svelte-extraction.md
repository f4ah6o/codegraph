# Port Liquid Vue and Svelte extraction

Created: 2026-05-17
Model: GPT-5 Codex

## 背景

original は Liquid、Vue、Svelte の template/component 抽出を持つ。frontend project の navigation では component と template include/render の関係が重要である。

## 期待する状態

- Liquid render/include/section を imports または references として抽出できる
- Vue/Svelte component nodes と script-level symbols を抽出できる
- route/component resolver が参照できる file-level metadata がある

## 実装メモ

- Reference original files: `src/extraction/liquid-extractor.ts`, `src/extraction/vue-extractor.ts`, `src/extraction/svelte-extractor.ts`
- Rust implementation area: `crates/codegraph/src/extraction*`, `crates/codegraph/tests/`
- embedded script extraction は段階的に扱う

## 検証

- Liquid/Vue/Svelte fixture tests
- `cargo test --all --all-features`

