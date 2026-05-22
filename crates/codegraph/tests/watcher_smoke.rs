use codegraph::config::CodeGraphConfig;
use codegraph::extraction::should_include_file;
use codegraph::watcher::should_watch_path;
use codegraph::CodeGraph;
use std::fs;
use tempfile::TempDir;

#[test]
fn watch_path_excludes_codegraph_dir() {
    let config = CodeGraphConfig::default_for_root(".");
    assert!(!should_watch_path(
        std::path::Path::new(".codegraph/codegraph.db"),
        &config
    ));
    assert!(!should_watch_path(
        std::path::Path::new(".codegraph/config.json"),
        &config
    ));
}

#[test]
fn watch_path_excludes_build_outputs() {
    let config = CodeGraphConfig::default_for_root(".");
    assert!(!should_watch_path(
        std::path::Path::new("target/debug/main"),
        &config
    ));
    assert!(!should_watch_path(
        std::path::Path::new("build/output.js"),
        &config
    ));
    assert!(!should_watch_path(
        std::path::Path::new("dist/bundle.js"),
        &config
    ));
}

#[test]
fn watch_path_includes_source_files() {
    let config = CodeGraphConfig::default_for_root(".");
    assert!(should_watch_path(
        std::path::Path::new("src/main.rs"),
        &config
    ));
    assert!(should_watch_path(
        std::path::Path::new("lib/app.ts"),
        &config
    ));
    assert!(should_watch_path(
        std::path::Path::new("src/lib.mbt"),
        &config
    ));
}

#[test]
fn watch_path_consistent_with_should_include_file() {
    let config = CodeGraphConfig::default_for_root(".");
    let included = [
        "src/lib.rs",
        "src/main.rs",
        "lib/index.ts",
        "app/main.py",
        "src/lib.mbt",
    ];
    let excluded = [
        ".codegraph/config.json",
        "target/debug/main",
        "build/output.js",
        "dist/bundle.js",
        "node_modules/react/index.js",
        ".git/HEAD",
        "README.md",
    ];
    for path in &included {
        assert_eq!(
            should_watch_path(std::path::Path::new(path), &config),
            should_include_file(std::path::Path::new(path), &config),
            "mismatch for {path}"
        );
    }
    for path in &excluded {
        assert!(
            !should_watch_path(std::path::Path::new(path), &config),
            "{path} should be excluded"
        );
    }
}

#[test]
fn watcher_config_default() {
    let config = codegraph::watcher::WatcherConfig::default();
    assert_eq!(config.debounce_ms, 300);
}

#[test]
fn sync_via_watcher_respects_config() {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("src")).unwrap();
    fs::write(dir.path().join("src/lib.rs"), "pub fn foo() {}\n").unwrap();
    fs::write(dir.path().join("README.md"), "# readme\n").unwrap();

    let mut cg = CodeGraph::init(dir.path()).unwrap();
    let initial = cg.index_all().unwrap();
    assert!(initial.success);
    assert_eq!(initial.files_indexed, 1);

    fs::write(
        dir.path().join("src/lib.rs"),
        "pub fn foo() {}\npub fn bar() { foo(); }\n",
    )
    .unwrap();

    let result = cg.sync().unwrap();
    assert!(result.success);
    assert!(result.files_indexed >= 1);
}
