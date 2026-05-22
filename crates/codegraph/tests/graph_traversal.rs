use codegraph::CodeGraph;
use tempfile::TempDir;

#[test]
fn graph_traversal_tracks_depth_and_suppresses_duplicates() {
    let dir = TempDir::new().unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("src/lib.rs"),
        r#"
pub fn entry() {
    middle();
    middle();
}

pub fn middle() {
    leaf();
}

pub fn sibling() {
    leaf();
}

pub fn leaf() {}
"#,
    )
    .unwrap();

    let mut cg = CodeGraph::init(dir.path()).unwrap();
    let index = cg.index_all().unwrap();
    assert!(index.success, "{:?}", index.errors);

    let leaf = first_node(&cg, "leaf");
    let callers = cg.get_callers(&leaf.id, 2).unwrap();
    let caller_names: Vec<_> = callers
        .iter()
        .map(|edge| (edge.node.name.as_str(), edge.depth))
        .collect();
    assert!(caller_names.contains(&("middle", 1)), "{caller_names:?}");
    assert!(caller_names.contains(&("sibling", 1)), "{caller_names:?}");
    assert!(caller_names.contains(&("entry", 2)), "{caller_names:?}");
    assert_eq!(
        caller_names
            .iter()
            .filter(|(name, _)| *name == "middle")
            .count(),
        1,
        "{caller_names:?}"
    );

    let entry = first_node(&cg, "entry");
    let paths = cg.find_paths(&entry.id, &leaf.id, 3, 5).unwrap();
    assert!(
        paths.iter().any(|path| {
            path.nodes
                .iter()
                .map(|node| node.name.as_str())
                .collect::<Vec<_>>()
                == ["entry", "middle", "leaf"]
        }),
        "{paths:?}"
    );
}

fn first_node(cg: &CodeGraph, symbol: &str) -> codegraph::types::Node {
    cg.search_nodes(symbol, Default::default())
        .unwrap()
        .into_iter()
        .find(|result| result.node.name == symbol)
        .unwrap()
        .node
}
