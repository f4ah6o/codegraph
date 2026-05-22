use serde_json::Value;
use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};
use tempfile::TempDir;

#[test]
fn mcp_lists_and_calls_status() {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("src")).unwrap();
    fs::write(
        dir.path().join("src/lib.rs"),
        "pub fn helper() {}\npub fn process_data() { helper(); }\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("src/lib.test.rs"),
        "pub fn test_process_data() {}\n",
    )
    .unwrap();

    let bin = env!("CARGO_BIN_EXE_cgz");
    assert!(Command::new(bin)
        .args(["init", dir.path().to_str().unwrap(), "--index"])
        .status()
        .unwrap()
        .success());

    let mut child = Command::new(bin)
        .args(["serve", "--mcp", "--path", dir.path().to_str().unwrap()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    {
        let stdin = child.stdin.as_mut().unwrap();
        writeln!(
            stdin,
            "{}",
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": { "rootUri": format!("file://{}", dir.path().display()) }
            })
        )
        .unwrap();
        writeln!(
            stdin,
            "{}",
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "tools/list"
            })
        )
        .unwrap();
        writeln!(
            stdin,
            "{}",
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "tools/call",
                "params": {
                    "name": "codegraph_status",
                    "arguments": {}
                }
            })
        )
        .unwrap();
        writeln!(
            stdin,
            "{}",
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": 4,
                "method": "tools/call",
                "params": {
                    "name": "codegraph_search",
                    "arguments": { "query": "process_data" }
                }
            })
        )
        .unwrap();
        writeln!(
            stdin,
            "{}",
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": 5,
                "method": "tools/call",
                "params": {
                    "name": "codegraph_context",
                    "arguments": {
                        "task": "change process_data behavior",
                        "format": "json",
                        "includeCode": false
                    }
                }
            })
        )
        .unwrap();
        writeln!(
            stdin,
            "{}",
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": 6,
                "method": "tools/call",
                "params": {
                    "name": "codegraph_affected",
                    "arguments": {
                        "files": ["src/lib.test.rs"]
                    }
                }
            })
        )
        .unwrap();
        writeln!(
            stdin,
            "{}",
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": 7,
                "method": "tools/call",
                "params": {
                    "name": "codegraph_files",
                    "arguments": {
                        "format": "tree",
                        "path": "src",
                        "pattern": "*.rs",
                        "includeMetadata": true,
                        "maxDepth": 2
                    }
                }
            })
        )
        .unwrap();
        writeln!(
            stdin,
            "{}",
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": 8,
                "method": "tools/call",
                "params": {
                    "name": "codegraph_explore",
                    "arguments": {
                        "query": "process_data",
                        "maxFiles": 2
                    }
                }
            })
        )
        .unwrap();
    }

    drop(child.stdin.take());
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let responses: Vec<Value> = stdout
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(responses[0]["result"]["serverInfo"]["name"], "codegraph");
    let tools = responses[1]["result"]["tools"].as_array().unwrap();
    assert!(tools.iter().any(|tool| tool["name"] == "codegraph_search"));
    assert!(tools
        .iter()
        .any(|tool| tool["name"] == "codegraph_affected"));
    let status_text = responses[2]["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    assert!(status_text.contains("Files indexed"));
    assert!(status_text.contains("Last indexed at"));
    assert!(status_text.contains("Stale files"));
    assert!(responses[3]["result"]["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("process_data"));
    let context_text = responses[4]["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    let context: Value = serde_json::from_str(context_text).unwrap();
    assert_eq!(context["query"], "change process_data behavior");
    assert!(context["symbols"]
        .as_array()
        .unwrap()
        .iter()
        .any(|symbol| symbol["name"] == "process_data"));
    let affected_text = responses[5]["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    let affected: Value = serde_json::from_str(affected_text).unwrap();
    assert!(affected["affectedTests"]
        .as_array()
        .unwrap()
        .iter()
        .any(|test| test == "src/lib.test.rs"));
    assert_eq!(
        affected["debug"][0]["matchedBy"]["directTestInput"],
        Value::Array(vec![Value::String("src/lib.test.rs".into())])
    );
    let files_text = responses[6]["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    assert!(files_text.contains("src/"), "{files_text}");
    assert!(files_text.contains("lib.rs (rust"), "{files_text}");
    assert!(files_text.contains("bytes"), "{files_text}");
    let explore_text = responses[7]["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    assert!(
        explore_text.contains("## Source Sections"),
        "{explore_text}"
    );
    assert!(
        explore_text.contains("## Relationship Map"),
        "{explore_text}"
    );
    assert!(explore_text.contains("Budget:"), "{explore_text}");
    assert!(explore_text.contains("process_data"), "{explore_text}");
}
