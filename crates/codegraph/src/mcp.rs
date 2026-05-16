use crate::types::{Node, NodeEdge, SearchOptions};
use crate::{find_nearest_codegraph_root, CodeGraph};
use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

const PROTOCOL_VERSION: &str = "2024-11-05";
const SERVER_INSTRUCTIONS: &str = "# Codegraph — code intelligence over an indexed knowledge graph\n\nStart with codegraph_status to check index health. Use codegraph_files, codegraph_search, codegraph_context, codegraph_callers/codegraph_callees, codegraph_impact, codegraph_node, and codegraph_explore for read-only exploration. Treat results as navigation context, not correctness proof; final validation still comes from the target repo's tests, type checks, linters, or build commands. Do not initialize or reindex a project unless the user explicitly asks for that workspace-changing action.";

pub struct MCPServer {
    project_path: Option<PathBuf>,
}

impl MCPServer {
    pub fn new(project_path: Option<PathBuf>) -> Self {
        Self { project_path }
    }

    pub fn start(&mut self) -> Result<()> {
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let response = match serde_json::from_str::<Value>(&line) {
                Ok(message) => self.handle_message(message),
                Err(_) => Some(error_response(
                    Value::Null,
                    -32700,
                    "Parse error: invalid JSON",
                )),
            };
            if let Some(response) = response {
                println!("{}", serde_json::to_string(&response)?);
                io::stdout().flush()?;
            }
        }
        Ok(())
    }

    fn handle_message(&mut self, message: Value) -> Option<Value> {
        let id = message.get("id").cloned();
        let method = message
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or_default();
        match method {
            "initialize" => {
                if let Some(path) = project_path_from_initialize(&message) {
                    self.project_path = Some(path);
                }
                id.map(|id| json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "protocolVersion": PROTOCOL_VERSION,
                        "capabilities": { "tools": {} },
                        "serverInfo": { "name": "codegraph", "version": env!("CARGO_PKG_VERSION") },
                        "instructions": SERVER_INSTRUCTIONS,
                    }
                }))
            }
            "initialized" => None,
            "tools/list" => id.map(|id| {
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": { "tools": tools() }
                })
            }),
            "tools/call" => {
                let Some(id) = id else { return None };
                let params = message.get("params").cloned().unwrap_or_else(|| json!({}));
                let name = params
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let args = params
                    .get("arguments")
                    .cloned()
                    .unwrap_or_else(|| json!({}));
                match self.execute_tool(name, &args) {
                    Ok(result) => Some(json!({ "jsonrpc": "2.0", "id": id, "result": result })),
                    Err(err) => Some(error_response(
                        id,
                        -32603,
                        &format!("Tool execution failed: {err}"),
                    )),
                }
            }
            "ping" => id.map(|id| json!({ "jsonrpc": "2.0", "id": id, "result": {} })),
            _ => id.map(|id| error_response(id, -32601, &format!("Method not found: {method}"))),
        }
    }

    fn execute_tool(&self, name: &str, args: &Value) -> Result<Value> {
        let cg = self.open_project(args)?;
        match name {
            "codegraph_search" => {
                let query = required_str(args, "query")?;
                let limit = clamp(
                    args.get("limit").and_then(Value::as_i64).unwrap_or(10),
                    1,
                    100,
                );
                let results = cg.search_nodes(
                    query,
                    SearchOptions {
                        limit,
                        ..Default::default()
                    },
                )?;
                if results.is_empty() {
                    Ok(text_result(format!("No results found for \"{query}\"")))
                } else {
                    let lines = results
                        .into_iter()
                        .map(|r| format_node(&r.node))
                        .collect::<Vec<_>>()
                        .join("\n");
                    Ok(text_result(lines))
                }
            }
            "codegraph_context" => {
                let task = required_str(args, "task")?;
                let max_nodes = clamp(
                    args.get("maxNodes").and_then(Value::as_i64).unwrap_or(20),
                    1,
                    200,
                );
                let include_code = args
                    .get("includeCode")
                    .and_then(Value::as_bool)
                    .unwrap_or(true);
                Ok(text_result(cg.build_context(
                    task,
                    max_nodes,
                    include_code,
                )?))
            }
            "codegraph_callers" => {
                let symbol = required_str(args, "symbol")?;
                let limit = clamp(
                    args.get("limit").and_then(Value::as_i64).unwrap_or(20),
                    1,
                    100,
                ) as usize;
                let nodes = find_matching_nodes(&cg, symbol)?;
                if nodes.is_empty() {
                    return Ok(text_result(format!(
                        "Symbol \"{symbol}\" not found in the codebase"
                    )));
                }
                let mut out = Vec::new();
                for node in nodes {
                    out.extend(cg.get_callers(&node.id, 1)?);
                }
                Ok(text_result(format_node_edges(
                    &format!("Callers of {symbol}"),
                    &out,
                    limit,
                )))
            }
            "codegraph_callees" => {
                let symbol = required_str(args, "symbol")?;
                let limit = clamp(
                    args.get("limit").and_then(Value::as_i64).unwrap_or(20),
                    1,
                    100,
                ) as usize;
                let nodes = find_matching_nodes(&cg, symbol)?;
                if nodes.is_empty() {
                    return Ok(text_result(format!(
                        "Symbol \"{symbol}\" not found in the codebase"
                    )));
                }
                let mut out = Vec::new();
                for node in nodes {
                    out.extend(cg.get_callees(&node.id, 1)?);
                }
                Ok(text_result(format_node_edges(
                    &format!("Callees of {symbol}"),
                    &out,
                    limit,
                )))
            }
            "codegraph_impact" => {
                let symbol = required_str(args, "symbol")?;
                let depth = clamp(
                    args.get("depth").and_then(Value::as_i64).unwrap_or(2),
                    1,
                    10,
                ) as usize;
                let nodes = find_matching_nodes(&cg, symbol)?;
                if nodes.is_empty() {
                    return Ok(text_result(format!(
                        "Symbol \"{symbol}\" not found in the codebase"
                    )));
                }
                let mut lines = vec![format!("## Impact: {symbol}")];
                for node in nodes {
                    let impact = cg.get_impact_radius(&node.id, depth)?;
                    for n in impact.nodes.values() {
                        lines.push(format!("- {}", format_node(n)));
                    }
                }
                Ok(text_result(lines.join("\n")))
            }
            "codegraph_node" => {
                let symbol = required_str(args, "symbol")?;
                let include_code = args
                    .get("includeCode")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                let nodes = find_matching_nodes(&cg, symbol)?;
                let Some(node) = nodes.first() else {
                    return Ok(text_result(format!(
                        "Symbol \"{symbol}\" not found in the codebase"
                    )));
                };
                let mut out = format_node(node);
                if include_code {
                    if let Ok(code) = cg.read_node_source(node) {
                        out.push_str("\n\n```");
                        out.push_str(node.language.as_str());
                        out.push('\n');
                        out.push_str(&code);
                        out.push_str("\n```");
                    }
                }
                Ok(text_result(out))
            }
            "codegraph_explore" => {
                let query = required_str(args, "query")?;
                let max_files = clamp(
                    args.get("maxFiles").and_then(Value::as_i64).unwrap_or(12),
                    1,
                    20,
                );
                let mut text = cg.build_context(query, max_files * 5, true)?;
                if text.len() > 35_000 {
                    text.truncate(35_000);
                    text.push_str("\n\n[truncated]");
                }
                Ok(text_result(text))
            }
            "codegraph_status" => {
                let stats = cg.stats()?;
                Ok(text_result(format!(
                    "**Files indexed:** {}\n**Nodes:** {}\n**Edges:** {}",
                    stats.file_count, stats.node_count, stats.edge_count
                )))
            }
            "codegraph_files" => {
                let path_filter = args.get("path").and_then(Value::as_str).unwrap_or("");
                let files = cg.get_all_files()?;
                let lines = files
                    .into_iter()
                    .filter(|f| path_filter.is_empty() || f.path.starts_with(path_filter))
                    .map(|f| format!("{} ({}, {} symbols)", f.path, f.language, f.node_count))
                    .collect::<Vec<_>>()
                    .join("\n");
                Ok(text_result(if lines.is_empty() {
                    "No files indexed. Run `codegraph index` first.".into()
                } else {
                    lines
                }))
            }
            _ => Err(anyhow!("Unknown tool: {name}")),
        }
    }

    fn open_project(&self, args: &Value) -> Result<CodeGraph> {
        if let Some(path) = args.get("projectPath").and_then(Value::as_str) {
            return CodeGraph::open(path);
        }
        let start = self
            .project_path
            .clone()
            .unwrap_or(std::env::current_dir()?);
        let root = find_nearest_codegraph_root(&start)
            .ok_or_else(|| anyhow!("CodeGraph not initialized in {}", start.display()))?;
        CodeGraph::open(root)
    }
}

fn tools() -> Value {
    json!([
        tool(
            "codegraph_search",
            "Quick symbol search by name.",
            json!({"query": {"type":"string"}, "kind": {"type":"string"}, "limit": {"type":"number"}, "projectPath": {"type":"string"}}),
            vec!["query"]
        ),
        tool(
            "codegraph_context",
            "Build comprehensive context for a task.",
            json!({"task": {"type":"string"}, "maxNodes": {"type":"number"}, "includeCode": {"type":"boolean"}, "projectPath": {"type":"string"}}),
            vec!["task"]
        ),
        tool(
            "codegraph_callers",
            "Find all functions/methods that call a specific symbol.",
            json!({"symbol": {"type":"string"}, "limit": {"type":"number"}, "projectPath": {"type":"string"}}),
            vec!["symbol"]
        ),
        tool(
            "codegraph_callees",
            "Find all functions/methods that a specific symbol calls.",
            json!({"symbol": {"type":"string"}, "limit": {"type":"number"}, "projectPath": {"type":"string"}}),
            vec!["symbol"]
        ),
        tool(
            "codegraph_impact",
            "Analyze the impact radius of changing a symbol.",
            json!({"symbol": {"type":"string"}, "depth": {"type":"number"}, "projectPath": {"type":"string"}}),
            vec!["symbol"]
        ),
        tool(
            "codegraph_node",
            "Get detailed information about a specific code symbol.",
            json!({"symbol": {"type":"string"}, "includeCode": {"type":"boolean"}, "projectPath": {"type":"string"}}),
            vec!["symbol"]
        ),
        tool(
            "codegraph_explore",
            "Deep exploration tool for a topic.",
            json!({"query": {"type":"string"}, "maxFiles": {"type":"number"}, "projectPath": {"type":"string"}}),
            vec!["query"]
        ),
        tool(
            "codegraph_status",
            "Get the status of the CodeGraph index.",
            json!({"projectPath": {"type":"string"}}),
            vec![]
        ),
        tool(
            "codegraph_files",
            "Get indexed project files.",
            json!({"path": {"type":"string"}, "pattern": {"type":"string"}, "format": {"type":"string"}, "includeMetadata": {"type":"boolean"}, "maxDepth": {"type":"number"}, "projectPath": {"type":"string"}}),
            vec![]
        ),
    ])
}

fn tool(name: &str, description: &str, properties: Value, required: Vec<&str>) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": {
            "type": "object",
            "properties": properties,
            "required": required,
        }
    })
}

fn project_path_from_initialize(message: &Value) -> Option<PathBuf> {
    let params = message.get("params")?;
    if let Some(uri) = params.get("rootUri").and_then(Value::as_str) {
        return Some(file_uri_to_path(uri));
    }
    params
        .get("workspaceFolders")
        .and_then(Value::as_array)
        .and_then(|folders| folders.first())
        .and_then(|folder| folder.get("uri"))
        .and_then(Value::as_str)
        .map(file_uri_to_path)
}

fn file_uri_to_path(uri: &str) -> PathBuf {
    let without_scheme = uri.strip_prefix("file://").unwrap_or(uri);
    PathBuf::from(percent_decode(without_scheme))
}

fn percent_decode(input: &str) -> String {
    let mut out = String::new();
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(hex) = u8::from_str_radix(&input[i + 1..i + 3], 16) {
                out.push(hex as char);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

fn required_str<'a>(args: &'a Value, key: &str) -> Result<&'a str> {
    args.get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("{key} must be a non-empty string"))
}

fn clamp(value: i64, min: i64, max: i64) -> i64 {
    value.max(min).min(max)
}

fn find_matching_nodes(cg: &CodeGraph, symbol: &str) -> Result<Vec<Node>> {
    Ok(cg
        .search_nodes(
            symbol,
            SearchOptions {
                limit: 50,
                ..Default::default()
            },
        )?
        .into_iter()
        .map(|r| r.node)
        .collect())
}

fn format_node(node: &Node) -> String {
    format!(
        "{} {} {}:{}",
        node.kind, node.name, node.file_path, node.start_line
    )
}

fn format_node_edges(title: &str, edges: &[NodeEdge], limit: usize) -> String {
    if edges.is_empty() {
        return format!("No results found for {title}");
    }
    let mut lines = vec![format!("## {title}")];
    for edge in edges.iter().take(limit) {
        lines.push(format!("- {}", format_node(&edge.node)));
    }
    lines.join("\n")
}

fn text_result(text: String) -> Value {
    json!({ "content": [{ "type": "text", "text": text }] })
}

fn error_response(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}
