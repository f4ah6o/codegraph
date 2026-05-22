use codegraph::types::SearchOptions;
use codegraph::CodeGraph;
use std::fs;
use tempfile::TempDir;

struct EvalCase {
    task: &'static str,
    expected_symbol: &'static str,
    expected_file: &'static str,
}

#[test]
fn agent_context_eval_reaches_expected_symbols_and_files() {
    let dir = TempDir::new().unwrap();
    write_eval_fixture(dir.path());

    let mut cg = CodeGraph::init(dir.path()).unwrap();
    let index = cg.index_all().unwrap();
    assert!(index.success, "{:?}", index.errors);

    let cases = [
        EvalCase {
            task: "change evict_expired cache policy behavior",
            expected_symbol: "evict_expired",
            expected_file: "src/cache.rs",
        },
        EvalCase {
            task: "inspect CacheStore configuration and lookup path",
            expected_symbol: "CacheStore",
            expected_file: "src/cache.rs",
        },
        EvalCase {
            task: "change parse_with_scheme validation for invalid scheme order",
            expected_symbol: "parse_with_scheme",
            expected_file: "parse.mbt",
        },
        EvalCase {
            task: "find MoonBit package tests affected by parse",
            expected_symbol: "parse",
            expected_file: "parse.mbt",
        },
    ];

    let mut passed = 0;
    for case in &cases {
        let report = cg.build_context_report(case.task, 20, false).unwrap();
        let found_symbol = report
            .symbols
            .iter()
            .any(|symbol| symbol.name == case.expected_symbol);
        let found_file = report
            .files
            .iter()
            .any(|file| file.path == case.expected_file);
        if found_symbol && found_file {
            passed += 1;
        } else {
            panic!(
                "task {:?} missed expected context: symbol={} file={} terms={:?} symbols={:?} files={:?}",
                case.task,
                found_symbol,
                found_file,
                report.search_terms,
                report
                    .symbols
                    .iter()
                    .map(|symbol| symbol.name.as_str())
                    .collect::<Vec<_>>(),
                report
                    .files
                    .iter()
                    .map(|file| file.path.as_str())
                    .collect::<Vec<_>>()
            );
        }
    }

    assert_eq!(passed, cases.len());
}

#[test]
fn search_ranking_prefers_exact_symbol_matches() {
    let dir = TempDir::new().unwrap();
    write_eval_fixture(dir.path());
    fs::write(
        dir.path().join("src/parse_helpers.rs"),
        "pub fn parse_with_scheme_helper() {}\n",
    )
    .unwrap();

    let mut cg = CodeGraph::init(dir.path()).unwrap();
    let index = cg.index_all().unwrap();
    assert!(index.success, "{:?}", index.errors);

    let results = cg
        .search_nodes(
            "parse_with_scheme",
            SearchOptions {
                limit: 5,
                ..Default::default()
            },
        )
        .unwrap();

    assert!(
        results.len() >= 2,
        "expected exact and prefix matches, got {results:?}"
    );
    assert_eq!(results[0].node.name, "parse_with_scheme");
    assert!(
        results[0].score > results[1].score,
        "expected exact match score above prefix/file matches: {results:?}"
    );
}

#[test]
fn explore_report_groups_source_relationships_and_budget() {
    let dir = TempDir::new().unwrap();
    write_eval_fixture(dir.path());

    let mut cg = CodeGraph::init(dir.path()).unwrap();
    let index = cg.index_all().unwrap();
    assert!(index.success, "{:?}", index.errors);

    let report = cg
        .build_explore_report("evict_expired CacheStore", 2)
        .unwrap();
    assert!(
        report
            .source_files
            .iter()
            .any(|file| file.path == "src/cache.rs"
                && file
                    .sections
                    .iter()
                    .any(|section| section.symbol == "evict_expired"
                        && section.code.contains("CachePolicy::EvictExpired"))),
        "{report:?}"
    );
    assert!(
        report.budget_guidance.contains("Small project"),
        "{report:?}"
    );
    assert!(
        report.source_files.len() <= 2,
        "source files should respect max_files: {report:?}"
    );
    assert!(!report.truncated, "{report:?}");
}

#[test]
fn affected_uses_rust_test_name_heuristic() {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("tokio/src/task")).unwrap();
    fs::create_dir_all(dir.path().join("tokio/tests")).unwrap();
    fs::write(
        dir.path().join("tokio/src/task/spawn.rs"),
        "pub fn spawn() {}\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("tokio/tests/task_spawn.rs"),
        "#[test]\nfn spawn_runs_task() {}\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("tokio/tests/time_driver.rs"),
        "#[test]\nfn time_driver_runs() {}\n",
    )
    .unwrap();

    let mut cg = CodeGraph::init(dir.path()).unwrap();
    let index = cg.index_all().unwrap();
    assert!(index.success, "{:?}", index.errors);

    let report = cg
        .build_affected_report(&["tokio/src/task/spawn.rs".to_string()])
        .unwrap();

    assert!(
        report
            .affected_tests
            .iter()
            .any(|test| test == "tokio/tests/task_spawn.rs"),
        "{report:?}"
    );
    assert!(
        report.debug[0]
            .matched_by
            .rust_name_heuristic
            .iter()
            .any(|test| test == "tokio/tests/task_spawn.rs"),
        "{report:?}"
    );
    assert!(
        !report
            .affected_tests
            .iter()
            .any(|test| test == "tokio/tests/time_driver.rs"),
        "{report:?}"
    );
}

#[test]
fn affected_uses_rust_workspace_heuristic() {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("crates/searcher/src/searcher")).unwrap();
    fs::create_dir_all(dir.path().join("crates/searcher/tests")).unwrap();
    fs::write(
        dir.path().join("crates/searcher/src/searcher/mod.rs"),
        "pub struct Searcher;\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("crates/searcher/src/searcher/glue.rs"),
        "pub fn glue() {}\n\n#[cfg(test)]\nmod tests { #[test] fn glue_unit() {} }\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("crates/searcher/tests/integration.rs"),
        "#[test]\nfn searcher_integration() {}\n",
    )
    .unwrap();
    fs::create_dir_all(dir.path().join("crates/other/tests")).unwrap();
    fs::write(
        dir.path().join("crates/other/tests/integration.rs"),
        "#[test]\nfn other_integration() {}\n",
    )
    .unwrap();

    let mut cg = CodeGraph::init(dir.path()).unwrap();
    let index = cg.index_all().unwrap();
    assert!(index.success, "{:?}", index.errors);

    let report = cg
        .build_affected_report(&["crates/searcher/src/searcher/mod.rs".to_string()])
        .unwrap();

    assert!(
        report
            .affected_tests
            .iter()
            .any(|test| test == "crates/searcher/tests/integration.rs"),
        "{report:?}"
    );
    assert!(
        report
            .affected_tests
            .iter()
            .any(|test| test == "crates/searcher/src/searcher/glue.rs"),
        "{report:?}"
    );
    assert!(
        report.debug[0]
            .matched_by
            .rust_workspace_heuristic
            .iter()
            .any(|test| test == "crates/searcher/tests/integration.rs"),
        "{report:?}"
    );
    assert!(
        !report
            .affected_tests
            .iter()
            .any(|test| test == "crates/other/tests/integration.rs"),
        "{report:?}"
    );
}

struct ExtractionExpectation {
    node_kind: &'static str,
    min_count: usize,
    must_contain: Vec<&'static str>,
}

struct SearchEvalCase {
    query: &'static str,
    expected_symbols: Vec<&'static str>,
}

struct ContextRecallCase {
    task: &'static str,
    expected_symbols: Vec<&'static str>,
    expected_files: Vec<&'static str>,
}

#[test]
fn extraction_coverage_captures_expected_node_kinds() {
    let dir = TempDir::new().unwrap();
    write_eval_fixture(dir.path());

    let mut cg = CodeGraph::init(dir.path()).unwrap();
    let index = cg.index_all().unwrap();
    assert!(index.success, "{:?}", index.errors);

    let stats = cg.stats().unwrap();
    let cases = [
        ExtractionExpectation {
            node_kind: "function",
            min_count: 2,
            must_contain: vec!["evict_expired", "lookup"],
        },
        ExtractionExpectation {
            node_kind: "struct",
            min_count: 1,
            must_contain: vec!["CacheStore"],
        },
        ExtractionExpectation {
            node_kind: "enum",
            min_count: 1,
            must_contain: vec!["CachePolicy"],
        },
        ExtractionExpectation {
            node_kind: "function",
            min_count: 2,
            must_contain: vec!["parse_with_scheme", "parse"],
        },
    ];

    for case in &cases {
        let nodes_by_kind: Vec<_> = stats
            .nodes_by_kind
            .iter()
            .filter(|(kind, _)| kind.as_str() == case.node_kind)
            .collect();
        let total: i64 = nodes_by_kind.iter().map(|(_, count)| *count).sum();
        assert!(
            total as usize >= case.min_count,
            "expected at least {} {} nodes, got {}",
            case.min_count,
            case.node_kind,
            total
        );

        for name in &case.must_contain {
            let results = cg
                .search_nodes(
                    name,
                    SearchOptions {
                        limit: 10,
                        ..Default::default()
                    },
                )
                .unwrap();
            assert!(
                results.iter().any(|r| r.node.name == *name),
                "expected to find {} as {} node but search returned: {:?}",
                name,
                case.node_kind,
                results
                    .iter()
                    .map(|r| format!("{}({})", r.node.name, r.node.kind))
                    .collect::<Vec<_>>()
            );
        }
    }
}

#[test]
fn search_recall_and_mrr_meets_thresholds() {
    let dir = TempDir::new().unwrap();
    write_eval_fixture(dir.path());

    let mut cg = CodeGraph::init(dir.path()).unwrap();
    let index = cg.index_all().unwrap();
    assert!(index.success, "{:?}", index.errors);

    let cases = [
        SearchEvalCase {
            query: "CacheStore",
            expected_symbols: vec!["CacheStore"],
        },
        SearchEvalCase {
            query: "evict_expired",
            expected_symbols: vec!["evict_expired", "EvictExpired"],
        },
        SearchEvalCase {
            query: "parse_with_scheme",
            expected_symbols: vec!["parse_with_scheme"],
        },
        SearchEvalCase {
            query: "parse",
            expected_symbols: vec!["parse", "parse_with_scheme"],
        },
    ];

    for case in &cases {
        let results = cg
            .search_nodes(
                case.query,
                SearchOptions {
                    limit: 10,
                    ..Default::default()
                },
            )
            .unwrap();

        let found_names: Vec<String> = results.iter().map(|r| r.node.name.clone()).collect();
        let recall = case
            .expected_symbols
            .iter()
            .filter(|sym| found_names.iter().any(|name| name == *sym))
            .count() as f64
            / case.expected_symbols.len() as f64;

        let mrr: f64 = case
            .expected_symbols
            .iter()
            .filter_map(|sym| {
                found_names
                    .iter()
                    .position(|name| name == *sym)
                    .map(|pos| 1.0 / (pos as f64 + 1.0))
            })
            .fold(f64::NEG_INFINITY, f64::max);
        let mrr = if mrr == f64::NEG_INFINITY { 0.0 } else { mrr };

        assert!(
            recall >= 0.5,
            "search recall for {:?} = {:.2} (below 0.5); found {:?}",
            case.query,
            recall,
            found_names
        );
        assert!(
            mrr > 0.0,
            "search MRR for {:?} = 0.0 (expected at least one expected symbol in results); found {:?}",
            case.query,
            found_names
        );
    }
}

#[test]
fn context_report_recall_meets_threshold() {
    let dir = TempDir::new().unwrap();
    write_eval_fixture(dir.path());

    let mut cg = CodeGraph::init(dir.path()).unwrap();
    let index = cg.index_all().unwrap();
    assert!(index.success, "{:?}", index.errors);

    let cases = [
        ContextRecallCase {
            task: "change evict_expired cache policy behavior",
            expected_symbols: vec!["evict_expired", "CachePolicy"],
            expected_files: vec!["src/cache.rs"],
        },
        ContextRecallCase {
            task: "inspect CacheStore configuration and lookup path",
            expected_symbols: vec!["CacheStore", "lookup"],
            expected_files: vec!["src/cache.rs"],
        },
        ContextRecallCase {
            task: "change parse_with_scheme validation for invalid scheme order",
            expected_symbols: vec!["parse_with_scheme"],
            expected_files: vec!["parse.mbt"],
        },
        ContextRecallCase {
            task: "find MoonBit package tests affected by parse",
            expected_symbols: vec!["parse"],
            expected_files: vec!["parse.mbt"],
        },
    ];

    for case in &cases {
        let report = cg.build_context_report(case.task, 20, false).unwrap();
        let symbol_names: Vec<String> = report.symbols.iter().map(|sym| sym.name.clone()).collect();
        let file_paths: Vec<String> = report.files.iter().map(|f| f.path.clone()).collect();

        let symbol_recall = case
            .expected_symbols
            .iter()
            .filter(|sym| symbol_names.iter().any(|name| name == *sym))
            .count() as f64
            / case.expected_symbols.len() as f64;

        let file_recall = case
            .expected_files
            .iter()
            .filter(|file| file_paths.iter().any(|path| path == *file))
            .count() as f64
            / case.expected_files.len() as f64;

        assert!(
            symbol_recall >= 0.5,
            "context symbol recall for {:?} = {:.2} (below 0.5); symbols={:?}",
            case.task,
            symbol_recall,
            symbol_names
        );
        assert!(
            file_recall >= 0.5,
            "context file recall for {:?} = {:.2} (below 0.5); files={:?}",
            case.task,
            file_recall,
            file_paths
        );
    }
}

#[test]
fn explore_report_covers_expected_symbols_and_relationships() {
    let dir = TempDir::new().unwrap();
    write_eval_fixture(dir.path());

    let mut cg = CodeGraph::init(dir.path()).unwrap();
    let index = cg.index_all().unwrap();
    assert!(index.success, "{:?}", index.errors);

    let report = cg
        .build_explore_report("CacheStore evict_expired", 4)
        .unwrap();

    let source_file_paths: Vec<&str> = report
        .source_files
        .iter()
        .map(|f| f.path.as_str())
        .collect();
    assert!(
        source_file_paths.iter().any(|path| path.contains("cache")),
        "explore report should include cache source file; got {:?}",
        source_file_paths
    );

    let section_symbols: Vec<String> = report
        .source_files
        .iter()
        .flat_map(|f| f.sections.iter().map(|s| s.symbol.clone()))
        .collect();
    assert!(
        section_symbols
            .iter()
            .any(|sym| sym == "evict_expired" || sym == "CacheStore"),
        "explore report should include evict_expired or CacheStore sections; got {:?}",
        section_symbols
    );

    if !report.relationships.is_empty() {
        let has_incoming_or_outgoing = report
            .relationships
            .iter()
            .any(|r| r.direction == "incoming" || r.direction == "outgoing");
        assert!(
            has_incoming_or_outgoing,
            "explore relationships should have incoming or outgoing direction; got {:?}",
            report.relationships
        );
    }

    assert!(
        !report.truncated || report.truncated_reason.is_some(),
        "truncated explore report should have a reason"
    );
}

fn write_eval_fixture(root: &std::path::Path) {
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("src/cache.rs"),
        r#"
pub struct CacheStore {
    entries: Vec<String>,
}

pub enum CachePolicy {
    KeepAll,
    EvictExpired,
}

pub fn evict_expired(store: &mut CacheStore, policy: CachePolicy) {
    match policy {
        CachePolicy::KeepAll => {}
        CachePolicy::EvictExpired => store.entries.clear(),
    }
}

pub fn lookup(store: &CacheStore, key: &str) -> Option<String> {
    store.entries.iter().find(|entry| entry.as_str() == key).cloned()
}
"#,
    )
    .unwrap();
    fs::write(root.join("moon.mod.json"), r#"{"name":"example/eval"}"#).unwrap();
    fs::write(root.join("moon.pkg.json"), "{}").unwrap();
    fs::write(
        root.join("parse.mbt"),
        r#"
pub fn parse_with_scheme(input : String) -> String {
  parse(input)
}

pub fn parse(input : String) -> String {
  input
}
"#,
    )
    .unwrap();
    fs::write(
        root.join("parse_test.mbt"),
        "test { inspect(parse(\"2026.5.3\"), content=\"2026.5.3\") }\n",
    )
    .unwrap();
}
