use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn cli_files_supports_tree_flat_grouped_filters_and_metadata() {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("src/bin")).unwrap();
    fs::create_dir_all(dir.path().join("tests")).unwrap();
    fs::write(dir.path().join("src/lib.rs"), "pub fn lib_symbol() {}\n").unwrap();
    fs::write(
        dir.path().join("src/bin/main.rs"),
        "fn main() { codegraph_fixture::lib_symbol(); }\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("tests/lib_test.rs"),
        "#[test] fn it_works() {}\n",
    )
    .unwrap();

    let bin = env!("CARGO_BIN_EXE_cgz");
    assert!(Command::new(bin)
        .args(["init", dir.path().to_str().unwrap(), "--index"])
        .status()
        .unwrap()
        .success());

    let tree = Command::new(bin)
        .args([
            "files",
            "--path",
            dir.path().to_str().unwrap(),
            "--format",
            "tree",
            "--filter-path",
            "src",
            "--max-depth",
            "3",
        ])
        .output()
        .unwrap();
    assert!(tree.status.success());
    let tree_text = String::from_utf8(tree.stdout).unwrap();
    assert!(tree_text.contains("src/"), "{tree_text}");
    assert!(tree_text.contains("lib.rs (rust"), "{tree_text}");
    assert!(!tree_text.contains("tests/"), "{tree_text}");

    let flat_json = Command::new(bin)
        .args([
            "files",
            "--path",
            dir.path().to_str().unwrap(),
            "--format",
            "flat",
            "--pattern",
            "*.rs",
            "--include-metadata",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(flat_json.status.success());
    let json: serde_json::Value = serde_json::from_slice(&flat_json.stdout).unwrap();
    assert_eq!(json["format"], "flat");
    assert!(json["files"].as_array().unwrap().iter().any(|file| {
        file["path"] == "src/lib.rs" && file["size"].as_u64().unwrap_or_default() > 0
    }));

    let legacy_json = Command::new(bin)
        .args(["files", "--path", dir.path().to_str().unwrap(), "--json"])
        .output()
        .unwrap();
    assert!(legacy_json.status.success());
    let legacy: serde_json::Value = serde_json::from_slice(&legacy_json.stdout).unwrap();
    assert!(legacy.as_array().unwrap().iter().any(|entry| {
        entry.as_array().unwrap()[0] == "rust"
            && entry.as_array().unwrap()[1].as_u64().unwrap() >= 3
    }));

    let grouped = Command::new(bin)
        .args([
            "files",
            "--path",
            dir.path().to_str().unwrap(),
            "--format",
            "grouped",
        ])
        .output()
        .unwrap();
    assert!(grouped.status.success());
    let grouped_text = String::from_utf8(grouped.stdout).unwrap();
    assert!(grouped_text.contains("rust:"), "{grouped_text}");
}
