use codegraph::types::SearchOptions;
use codegraph::CodeGraph;
use std::fs;
use tempfile::TempDir;

#[test]
fn sync_skips_unchanged_indexes_changed_and_deletes_removed_files() {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("src")).unwrap();
    fs::write(dir.path().join("src/lib.rs"), "pub fn keep() {}\n").unwrap();
    fs::write(dir.path().join("src/old.rs"), "pub fn old_symbol() {}\n").unwrap();

    let mut cg = CodeGraph::init(dir.path()).unwrap();
    let initial = cg.index_all().unwrap();
    assert!(initial.success, "{initial:?}");
    assert_eq!(initial.files_indexed, 2);

    let unchanged = cg.sync().unwrap();
    assert!(unchanged.success, "{unchanged:?}");
    assert_eq!(unchanged.files_indexed, 0, "{unchanged:?}");
    assert_eq!(unchanged.files_skipped, 2, "{unchanged:?}");
    assert_eq!(unchanged.files_deleted, 0, "{unchanged:?}");

    fs::write(
        dir.path().join("src/lib.rs"),
        "pub fn keep() {}\npub fn changed_symbol() { keep(); }\n",
    )
    .unwrap();
    fs::remove_file(dir.path().join("src/old.rs")).unwrap();
    fs::write(dir.path().join("src/new.rs"), "pub fn new_symbol() {}\n").unwrap();

    let changed = cg.sync().unwrap();
    assert!(changed.success, "{changed:?}");
    assert_eq!(changed.files_indexed, 2, "{changed:?}");
    assert_eq!(changed.files_skipped, 0, "{changed:?}");
    assert_eq!(changed.files_deleted, 1, "{changed:?}");

    let files = cg.get_all_files().unwrap();
    assert!(files.iter().any(|file| file.path == "src/lib.rs"));
    assert!(files.iter().any(|file| file.path == "src/new.rs"));
    assert!(!files.iter().any(|file| file.path == "src/old.rs"));

    assert!(cg
        .search_nodes(
            "changed_symbol",
            SearchOptions {
                limit: 5,
                ..Default::default()
            },
        )
        .unwrap()
        .iter()
        .any(|result| result.node.name == "changed_symbol"));
    assert!(cg
        .search_nodes(
            "new_symbol",
            SearchOptions {
                limit: 5,
                ..Default::default()
            },
        )
        .unwrap()
        .iter()
        .any(|result| result.node.name == "new_symbol"));
    assert!(cg
        .search_nodes(
            "old_symbol",
            SearchOptions {
                limit: 5,
                ..Default::default()
            },
        )
        .unwrap()
        .is_empty());
}
