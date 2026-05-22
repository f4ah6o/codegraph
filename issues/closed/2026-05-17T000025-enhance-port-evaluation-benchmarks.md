# Port evaluation benchmarks

Created: 2026-05-17
Completed: 2026-05-23
Model: GPT-5 Codex

## 背景

original は CodeGraph が agent exploration を改善することを evaluation と benchmark で確認している。Rust `cgz` でも extraction/context quality の改善を定量的に追える軽量 evaluation が必要である。

## 期待する状態

- controlled fixtures に対して context が必要 symbol/file を含むか検証できる
- extraction coverage と context quality の regressions を CI で検出できる
- marketing claim ではなく、開発用の engineering metric として扱う

## 実装メモ

- Reference original files: `__tests__/evaluation/**`, `docs/SEARCH_QUALITY_LOOP.md`, `README.md`
- Rust implementation area: `crates/codegraph/tests/`, `docs/`
- large external repositories に依存する test は標準 test suite に入れない

## 解決方法

Added four new test functions to `agent_context_eval.rs` and updated documentation:

### New tests in `crates/codegraph/tests/agent_context_eval.rs`

1. **`extraction_coverage_captures_expected_node_kinds`** - Indexes the shared fixture and asserts that expected node kinds appear with minimum counts and are retrievable via `search_nodes`.

2. **`search_recall_and_mrr_meets_thresholds`** - For predefined search queries, computes per-case recall and MRR. Asserts recall >= 0.5 and MRR > 0 for each case.

3. **`context_report_recall_meets_threshold`** - For task-oriented context queries, computes per-case symbol recall and file recall. Asserts each recall >= 0.5.

4. **`explore_report_covers_expected_symbols_and_relationships`** - Verifies `build_explore_report` includes source files matching the query topic, sections with expected symbol names, and that any reported relationships have valid direction tags.

### Updated `docs/AGENT_CONTEXT_EVAL.md`

Replaced marketing-style language with an engineering metrics table covering extraction coverage, search recall, search MRR, context symbol recall, context file recall, and explore coverage. Documented all test functions and their thresholds. Added instructions for adding new cases.

### Updated `docs/PORT_PARITY_MATRIX.md`

Changed the Evaluation benchmarks row status from Partial to Done and updated the notes to reflect the full suite of per-case threshold tests.

## 検証

- Evaluation fixture tests
- `cargo test --all --all-features`
