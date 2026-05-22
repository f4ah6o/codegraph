# Agent Context Evaluation

`cgz` measures extraction coverage and context quality with engineering metrics,
not marketing claims. The evaluation suite encodes quantifiable thresholds so
regressions are detected by `cargo test`.

## Metrics

| Metric | Definition | Threshold |
| --- | --- | --- |
| Extraction coverage | Node kinds present after indexing a known fixture | Each expected node kind appears with at least the minimum count |
| Search recall | Fraction of expected symbols found in search results | >= 0.5 per case |
| Search MRR | Reciprocal rank of the first expected symbol in search results | > 0 per case |
| Context symbol recall | Fraction of expected symbols in `build_context_report` output | >= 0.5 per case |
| Context file recall | Fraction of expected files in `build_context_report` output | >= 0.5 per case |
| Explore coverage | `build_explore_report` includes expected source files and symbol sections | Source file and section symbols present |

All thresholds are per-case minimums. A case failing any threshold causes the
test to fail, making regressions visible in CI.

## Automated Fixture

The Rust integration test `agent_context_eval` creates a small mixed Rust and
MoonBit fixture, indexes it, and runs the following test functions:

- **`agent_context_eval_reaches_expected_symbols_and_files`** - task-oriented
  context report checks that expected symbols and files both appear.
- **`search_ranking_prefers_exact_symbol_matches`** - exact match scores above
  prefix and file-path matches.
- **`explore_report_groups_source_relationships_and_budget`** - explore report
  contains expected source sections, relationships, and budget guidance.
- **`affected_uses_rust_test_name_heuristic`** and
  **`affected_uses_rust_workspace_heuristic`** - test-impact analysis heuristics.
- **`extraction_coverage_captures_expected_node_kinds`** - verifies indexed
  fixture has expected node kind counts and that expected symbol names are
  retrievable via search.
- **`search_recall_and_mrr_meets_thresholds`** - computes per-case recall and
  MRR for `search_nodes` and asserts minimum quality.
- **`context_report_recall_meets_threshold`** - computes per-case symbol and
  file recall for `build_context_report` and asserts minimum quality.
- **`explore_report_covers_expected_symbols_and_relationships`** - verifies
  `build_explore_report` includes expected source symbols and direction-tagged
  relationships.

Run the suite with:

```bash
cargo test -p cgz --test agent_context_eval
```

## Adding Cases

New cases should model agent tasks rather than raw symbol lookups. Each case
names the expected symbols and files so regressions are actionable. When adding
a case:

1. Add the case definition to the relevant test function.
2. Set expected symbols/files to the minimum set an agent needs.
3. Ensure the threshold is explicit (recall >= 0.5 or MRR > 0).
4. Run `cargo test -p cgz --test agent_context_eval` to verify.

For real repositories, use the same pattern: define task, expected symbols,
expected files, and recall/MRR thresholds before changing ranking or extraction.
