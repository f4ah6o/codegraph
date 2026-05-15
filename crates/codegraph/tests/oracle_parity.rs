use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn ts_oracle_bin() -> PathBuf {
    std::env::var_os("CODEGRAPH_TS_ORACLE_BIN")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_root().join("dist/bin/codegraph.js"))
}

fn ensure_ts_oracle() {
    let bin = ts_oracle_bin();
    if bin.exists() {
        return;
    }
    if !repo_root().join("node_modules/.bin/tsc").exists() {
        let status = Command::new("npm")
            .arg("install")
            .current_dir(repo_root())
            .status()
            .expect("failed to run npm install for TypeScript oracle");
        assert!(
            status.success(),
            "npm install failed for TypeScript oracle dependencies"
        );
    }
    let status = Command::new("npm")
        .arg("run")
        .arg("build")
        .current_dir(repo_root())
        .status()
        .expect("failed to run npm build for TypeScript oracle");
    assert!(status.success(), "npm build failed for TypeScript oracle");
}

fn run_ts(args: &[&str]) -> String {
    let output = Command::new("node")
        .arg(ts_oracle_bin())
        .args(args)
        .env("CODEGRAPH_ALLOW_UNSAFE_NODE", "1")
        .output()
        .expect("failed to run TypeScript oracle");
    assert!(
        output.status.success(),
        "TypeScript oracle failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

fn run_rust(args: &[&str]) -> String {
    let bin = env!("CARGO_BIN_EXE_codegraph");
    let output = Command::new(bin)
        .args(args)
        .output()
        .expect("failed to run Rust codegraph");
    assert!(
        output.status.success(),
        "Rust codegraph failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

fn copy_dir(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).unwrap();
    for entry in walkdir::WalkDir::new(src) {
        let entry = entry.unwrap();
        let rel = entry.path().strip_prefix(src).unwrap();
        if rel.as_os_str().is_empty() {
            continue;
        }
        let target = dst.join(rel);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target).unwrap();
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

fn write_fixture(root: &Path) {
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("src/lib.rs"),
        r#"
pub struct User {
    pub id: String,
}

pub const DEFAULT_LIMIT: usize = 10;

pub trait Repository {
    fn find(&self, id: &str) -> Option<User>;
}

pub enum Status {
    Active,
    Disabled,
}

pub type UserId = String;

pub fn process_data(input: &str) -> String {
    helper(input)
}

fn helper(input: &str) -> String {
    input.to_string()
}

impl Repository for User {
    fn find(&self, _id: &str) -> Option<User> {
        None
    }
}
"#,
    )
    .unwrap();
}

fn normalize_status_ts(value: Value) -> Value {
    serde_json::json!({
        "file_count": value["fileCount"],
        "languages": value["languages"],
    })
}

fn normalize_status_rust(value: Value) -> Value {
    let languages: Vec<Value> = value["files_by_language"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry[0].clone())
        .collect();
    serde_json::json!({
        "file_count": value["file_count"],
        "languages": languages,
    })
}

fn normalize_query(value: Value) -> Vec<(String, String, String)> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|r| {
            let node = &r["node"];
            (
                node["kind"].as_str().unwrap().to_string(),
                node["name"].as_str().unwrap().to_string(),
                node["filePath"]
                    .as_str()
                    .or_else(|| node["file_path"].as_str())
                    .unwrap()
                    .to_string(),
            )
        })
        .collect()
}

#[test]
fn rust_port_matches_typescript_oracle_for_basic_rust_fixture() {
    ensure_ts_oracle();

    let base = TempDir::new().unwrap();
    let fixture = base.path().join("fixture");
    write_fixture(&fixture);

    let ts_project = base.path().join("ts-project");
    let rust_project = base.path().join("rust-project");
    copy_dir(&fixture, &ts_project);
    copy_dir(&fixture, &rust_project);

    run_ts(&["init", ts_project.to_str().unwrap()]);
    run_ts(&["index", ts_project.to_str().unwrap(), "--quiet"]);
    run_rust(&["init", rust_project.to_str().unwrap()]);
    run_rust(&["index", rust_project.to_str().unwrap(), "--quiet"]);

    let ts_status: Value =
        serde_json::from_str(&run_ts(&["status", ts_project.to_str().unwrap(), "--json"])).unwrap();
    let rust_status: Value = serde_json::from_str(&run_rust(&[
        "status",
        rust_project.to_str().unwrap(),
        "--json",
    ]))
    .unwrap();
    assert_eq!(
        normalize_status_rust(rust_status),
        normalize_status_ts(ts_status)
    );

    let ts_query: Value = serde_json::from_str(&run_ts(&[
        "query",
        "process_data",
        "--path",
        ts_project.to_str().unwrap(),
        "--json",
    ]))
    .unwrap();
    let rust_query: Value = serde_json::from_str(&run_rust(&[
        "query",
        "process_data",
        "--path",
        rust_project.to_str().unwrap(),
        "--json",
    ]))
    .unwrap();
    assert_eq!(normalize_query(rust_query), normalize_query(ts_query));

    let rust_status_query: Value = serde_json::from_str(&run_rust(&[
        "query",
        "Status",
        "--path",
        rust_project.to_str().unwrap(),
        "--json",
    ]))
    .unwrap();
    let status_symbols = normalize_query(rust_status_query);
    assert!(status_symbols
        .iter()
        .any(|(kind, name, _)| kind == "enum" && name == "Status"));

    let rust_find_query: Value = serde_json::from_str(&run_rust(&[
        "query",
        "find",
        "--path",
        rust_project.to_str().unwrap(),
        "--json",
    ]))
    .unwrap();
    let find_symbols = normalize_query(rust_find_query);
    assert!(find_symbols
        .iter()
        .any(|(kind, name, _)| kind == "method" && name == "find"));
}
