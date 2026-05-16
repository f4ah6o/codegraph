use serde_json::Value;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn run(args: &[&str]) -> String {
    let bin = env!("CARGO_BIN_EXE_cgz");
    let output = Command::new(bin)
        .args(args)
        .output()
        .expect("failed to run codegraph");
    assert!(
        output.status.success(),
        "codegraph failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

fn query(project: &str, term: &str) -> Vec<(String, String, i64)> {
    let value: Value =
        serde_json::from_str(&run(&["query", term, "--path", project, "--json"])).unwrap();
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|r| {
            let node = &r["node"];
            (
                node["kind"].as_str().unwrap().to_string(),
                node["name"].as_str().unwrap().to_string(),
                node["start_line"].as_i64().unwrap(),
            )
        })
        .collect()
}

#[test]
fn moonbit_tree_sitter_extracts_symbols_and_metadata() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("moon.mod.json"),
        r#"{"name":"example/graph"}"#,
    )
    .unwrap();
    fs::write(dir.path().join("moon.pkg.json"), "{}").unwrap();
    fs::write(
        dir.path().join("lib.mbt"),
        r#"
pub struct User {
  id : String
}

pub enum Status {
  Active
  Disabled
}

pub trait Repository {
  find(Self, String) -> User?
}

pub fn process_data(input : String) -> String {
  helper(input)
}

fn helper(input : String) -> String {
  input
}

impl Repository for User with find(self, id) {
  None
}
"#,
    )
    .unwrap();
    fs::write(
        dir.path().join("README.mbt.md"),
        r#"# Example

```mbt
pub fn documented(value : String) -> String {
  value
}
```
"#,
    )
    .unwrap();

    let project = dir.path().to_str().unwrap();
    run(&["init", project, "--index"]);

    let process = query(project, "process_data");
    assert!(process
        .iter()
        .any(|(kind, name, _)| kind == "function" && name == "process_data"));

    let find = query(project, "find");
    assert!(find
        .iter()
        .any(|(kind, name, _)| kind == "method" && name == "find"));

    let active = query(project, "Active");
    assert!(active
        .iter()
        .any(|(kind, name, _)| kind == "enum_member" && name == "Active"));

    let documented = query(project, "documented");
    assert!(documented
        .iter()
        .any(|(kind, name, line)| kind == "function" && name == "documented" && *line == 4));

    let module = query(project, "example/graph");
    assert!(module
        .iter()
        .any(|(kind, name, _)| kind == "module" && name == "example/graph"));
}

#[test]
fn context_extracts_symbol_terms_from_natural_language() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("moon.mod.json"),
        r#"{"name":"example/calver"}"#,
    )
    .unwrap();
    fs::write(dir.path().join("moon.pkg.json"), "{}").unwrap();
    fs::write(
        dir.path().join("parse.mbt"),
        r#"
pub fn parse_with_scheme(input : String) -> String {
  input
}

pub fn parse(input : String) -> String {
  parse_with_scheme(input)
}
"#,
    )
    .unwrap();

    let project = dir.path().to_str().unwrap();
    run(&["init", project, "--index"]);

    let output = run(&[
        "context",
        "change parse_with_scheme validation for invalid scheme order",
        "--path",
        project,
    ]);
    assert!(output.contains("parse_with_scheme"), "{output}");
    assert!(output.contains("pub fn parse_with_scheme"), "{output}");
}

#[test]
fn context_guides_when_no_symbols_match() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("moon.mod.json"),
        r#"{"name":"example/empty"}"#,
    )
    .unwrap();
    fs::write(dir.path().join("moon.pkg.json"), "{}").unwrap();
    fs::write(dir.path().join("lib.mbt"), "pub fn known_symbol() -> Int { 1 }\n").unwrap();

    let project = dir.path().to_str().unwrap();
    run(&["init", project, "--index"]);

    let output = run(&["context", "zzzz_no_matching_symbol", "--path", project]);
    assert!(output.contains("No matching symbols or files were found"), "{output}");
    assert!(output.contains("cgz query --json"), "{output}");
}
