# Agent Context Evaluation

`cgz` is optimized for agent code navigation. Its primary product metric is
whether an agent can reach the right files and symbols from the first few
context queries.

## Automated Fixture

The Rust integration test `agent_context_eval` creates a small mixed Rust and
MoonBit fixture, indexes it, and checks that representative natural-language
tasks reach the expected symbol and file through `CodeGraph::build_context_report`.

Run it with:

```bash
cargo test -p cgz --test agent_context_eval
```

The test currently covers:

- Rust cache task to `evict_expired` in `src/cache.rs`
- Rust type lookup task to `CacheStore` in `src/cache.rs`
- MoonBit validation task to `parse_with_scheme` in `parse.mbt`
- MoonBit parse/test planning task to `parse` in `parse.mbt`

## Success Criteria

A case passes when both the expected symbol and expected file appear in the
structured context report. New cases should model agent tasks rather than raw
symbol lookups, and each case should name the expected symbol and file so
regressions are actionable.

For real repositories, use the same pattern: define task, expected symbols,
expected files, and a pass/fail result before changing ranking or extraction.
