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
fn context_extracts_short_camel_case_type_from_natural_language() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("moon.mod.json"),
        r#"{"name":"example/json"}"#,
    )
    .unwrap();
    fs::write(dir.path().join("moon.pkg.json"), "{}").unwrap();
    fs::write(
        dir.path().join("json.mbt"),
        r#"
pub enum Json {
  String(String)
  Number(Double)
}

pub fn stringify(value : Json) -> String {
  ""
}
"#,
    )
    .unwrap();
    fs::write(
        dir.path().join("show.mbt"),
        r#"
pub trait Show {
  output(Self, Logger) -> Unit
}
"#,
    )
    .unwrap();

    let project = dir.path().to_str().unwrap();
    run(&["init", project, "--index"]);

    let output = run(&[
        "context",
        "How is Json implemented?",
        "--path",
        project,
        "--json",
    ]);
    let value: Value = serde_json::from_str(&output).unwrap();
    let search_terms = value["search_terms"].as_array().unwrap();
    assert!(!search_terms.iter().any(|term| term == "How"), "{output}");
    assert!(search_terms.iter().any(|term| term == "Json"), "{output}");
    assert_eq!(value["matches"][0]["node"]["name"], "Json", "{output}");
    assert_eq!(
        value["matches"][0]["node"]["file_path"], "json.mbt",
        "{output}"
    );
}

#[test]
fn context_json_returns_agent_friendly_evidence() {
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
        "--json",
    ]);
    let value: Value = serde_json::from_str(&output).unwrap();

    assert_eq!(
        value["query"].as_str().unwrap(),
        "change parse_with_scheme validation for invalid scheme order"
    );
    assert!(value["search_terms"]
        .as_array()
        .unwrap()
        .iter()
        .any(|term| term == "parse_with_scheme"));

    let matches = value["matches"].as_array().unwrap();
    assert!(matches.iter().any(|entry| {
        entry["search_term"] == "parse_with_scheme"
            && entry["reason"]
                .as_str()
                .unwrap()
                .contains("extracted task term")
            && entry["node"]["name"] == "parse_with_scheme"
            && entry["code"]
                .as_str()
                .unwrap()
                .contains("pub fn parse_with_scheme")
    }));

    assert!(value["files"].as_array().unwrap().iter().any(|entry| {
        entry["path"] == "parse.mbt"
            && entry["symbols"]
                .as_array()
                .unwrap()
                .iter()
                .any(|s| s == "parse_with_scheme")
    }));
    assert!(value["symbols"].as_array().unwrap().iter().any(|entry| {
        entry["name"] == "parse_with_scheme"
            && entry["kind"] == "function"
            && entry["file_path"] == "parse.mbt"
    }));
    assert!(value["warnings"].as_array().unwrap().is_empty());
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
    fs::write(
        dir.path().join("lib.mbt"),
        "pub fn known_symbol() -> Int { 1 }\n",
    )
    .unwrap();

    let project = dir.path().to_str().unwrap();
    run(&["init", project, "--index"]);

    let output = run(&["context", "zzzz_no_matching_symbol", "--path", project]);
    assert!(
        output.contains("No matching symbols or files were found"),
        "{output}"
    );
    assert!(output.contains("cgz query --json"), "{output}");
}

#[test]
fn affected_includes_moonbit_same_package_tests() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("moon.mod.json"),
        r#"{"name":"example/calver"}"#,
    )
    .unwrap();
    fs::write(dir.path().join("moon.pkg.json"), "{}").unwrap();
    fs::write(
        dir.path().join("parse.mbt"),
        "pub fn parse() -> Int { 1 }\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("scheme.mbt"),
        "pub fn scheme() -> Int { 2 }\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("parse_test.mbt"),
        "test { inspect(parse(), content=\"1\") }\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("parse_wbtest.mbt"),
        "test { inspect(scheme(), content=\"2\") }\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("README.mbt.md"),
        "# Example\n\n```mbt check\ntest { inspect(parse(), content=\"1\") }\n```\n",
    )
    .unwrap();
    fs::create_dir_all(dir.path().join("other")).unwrap();
    fs::write(dir.path().join("other/moon.pkg.json"), "{}").unwrap();
    fs::write(
        dir.path().join("other/other_test.mbt"),
        "test { inspect(1, content=\"1\") }\n",
    )
    .unwrap();

    let project = dir.path().to_str().unwrap();
    run(&["init", project, "--index"]);

    let output = run(&[
        "affected",
        "parse.mbt",
        "scheme.mbt",
        "--path",
        project,
        "--json",
    ]);
    let value: Value = serde_json::from_str(&output).unwrap();
    let tests = value["affectedTests"].as_array().unwrap();
    assert!(tests.iter().any(|v| v == "README.mbt.md"), "{output}");
    assert!(tests.iter().any(|v| v == "parse_test.mbt"), "{output}");
    assert!(tests.iter().any(|v| v == "parse_wbtest.mbt"), "{output}");
    assert!(
        !tests.iter().any(|v| v == "other/other_test.mbt"),
        "{output}"
    );
    assert!(
        value["debug"].as_array().unwrap().iter().any(|entry| {
            entry["reason"]
                .as_str()
                .unwrap()
                .contains("MoonBit same-package")
        }),
        "{output}"
    );
    assert!(
        value["debug"].as_array().unwrap().iter().any(|entry| {
            entry["changedFile"] == "parse.mbt"
                && entry["matchedBy"]["moonbitSamePackage"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|test| test == "parse_test.mbt")
                && entry["matchedBy"]["moonbitSamePackage"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|test| test == "README.mbt.md")
        }),
        "{output}"
    );
    assert!(value["warnings"].as_array().unwrap().is_empty(), "{output}");
}

#[test]
fn affected_includes_moonbit_dependent_package_tests() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("moon.mod.json"),
        r#"{"name":"example/app"}"#,
    )
    .unwrap();
    fs::write(
        dir.path().join("moon.pkg.json"),
        r#"{"import":["example/app/runtime"]}"#,
    )
    .unwrap();
    fs::write(
        dir.path().join("app.mbt"),
        "pub fn run() -> Int { @runtime.vm() }\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("app_test.mbt"),
        "test { inspect(run(), content=\"1\") }\n",
    )
    .unwrap();
    fs::create_dir_all(dir.path().join("runtime")).unwrap();
    fs::write(
        dir.path().join("runtime/moon.pkg.json"),
        r#"{"import":["example/app/engine"]}"#,
    )
    .unwrap();
    fs::write(
        dir.path().join("runtime/runtime_state.mbt"),
        "pub fn vm() -> Int { @engine.parse() }\n",
    )
    .unwrap();
    fs::create_dir_all(dir.path().join("engine")).unwrap();
    fs::write(dir.path().join("engine/moon.pkg.json"), "{}").unwrap();
    fs::write(
        dir.path().join("engine/parser.mbt"),
        "pub fn parse() -> Int { 1 }\n",
    )
    .unwrap();
    fs::create_dir_all(dir.path().join("other")).unwrap();
    fs::write(dir.path().join("other/moon.pkg.json"), "{}").unwrap();
    fs::write(
        dir.path().join("other/other_test.mbt"),
        "test { inspect(1, content=\"1\") }\n",
    )
    .unwrap();

    let project = dir.path().to_str().unwrap();
    run(&["init", project, "--index"]);

    let runtime_output = run(&[
        "affected",
        "runtime/runtime_state.mbt",
        "--path",
        project,
        "--json",
    ]);
    let runtime: Value = serde_json::from_str(&runtime_output).unwrap();
    assert!(
        runtime["affectedTests"]
            .as_array()
            .unwrap()
            .iter()
            .any(|test| test == "app_test.mbt"),
        "{runtime_output}"
    );
    assert!(
        runtime["debug"].as_array().unwrap().iter().any(|entry| {
            entry["changedFile"] == "runtime/runtime_state.mbt"
                && entry["matchedBy"]["moonbitPackageDependents"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|test| test == "app_test.mbt")
        }),
        "{runtime_output}"
    );
    assert!(
        !runtime["affectedTests"]
            .as_array()
            .unwrap()
            .iter()
            .any(|test| test == "other/other_test.mbt"),
        "{runtime_output}"
    );

    let engine_output = run(&["affected", "engine/parser.mbt", "--path", project, "--json"]);
    let engine: Value = serde_json::from_str(&engine_output).unwrap();
    assert!(
        engine["affectedTests"]
            .as_array()
            .unwrap()
            .iter()
            .any(|test| test == "app_test.mbt"),
        "{engine_output}"
    );
    assert!(
        engine["debug"].as_array().unwrap().iter().any(|entry| {
            entry["changedFile"] == "engine/parser.mbt"
                && entry["matchedBy"]["moonbitPackageDependents"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|test| test == "app_test.mbt")
        }),
        "{engine_output}"
    );
    assert!(
        engine["warnings"].as_array().unwrap().is_empty(),
        "{engine_output}"
    );
}

#[test]
fn affected_keeps_direct_moonbit_test_input() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("moon.mod.json"),
        r#"{"name":"example/calver"}"#,
    )
    .unwrap();
    fs::write(dir.path().join("moon.pkg.json"), "{}").unwrap();
    fs::write(
        dir.path().join("parse_test.mbt"),
        "test { inspect(1, content=\"1\") }\n",
    )
    .unwrap();

    let project = dir.path().to_str().unwrap();
    run(&["init", project, "--index"]);

    let output = run(&["affected", "parse_test.mbt", "--path", project, "--json"]);
    let value: Value = serde_json::from_str(&output).unwrap();
    let tests = value["affectedTests"].as_array().unwrap();
    assert_eq!(tests, &[Value::String("parse_test.mbt".into())]);
    let debug = &value["debug"].as_array().unwrap()[0];
    assert_eq!(
        debug["matchedBy"]["directTestInput"],
        Value::Array(vec![Value::String("parse_test.mbt".into())])
    );
}
