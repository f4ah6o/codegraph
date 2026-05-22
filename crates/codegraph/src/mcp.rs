use crate::types::{
    FileListFormat, FileListOptions, FileListReport, Node, NodeEdge, SearchOptions,
};
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
                if args.get("format").and_then(Value::as_str) == Some("json") {
                    Ok(text_result(serde_json::to_string_pretty(
                        &cg.build_context_report(task, max_nodes, include_code)?,
                    )?))
                } else {
                    Ok(text_result(cg.build_context(
                        task,
                        max_nodes,
                        include_code,
                    )?))
                }
            }
            "codegraph_callers" => {
                let symbol = required_str(args, "symbol")?;
                let limit = clamp(
                    args.get("limit").and_then(Value::as_i64).unwrap_or(20),
                    1,
                    100,
                ) as usize;
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
                let mut out = Vec::new();
                for node in nodes {
                    out.extend(cg.get_callers(&node.id, depth)?);
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
                let mut out = Vec::new();
                for node in nodes {
                    out.extend(cg.get_callees(&node.id, depth)?);
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
                let limit = clamp(
                    args.get("limit").and_then(Value::as_i64).unwrap_or(50),
                    1,
                    200,
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
                    let mut impact_nodes = impact.nodes.into_values().collect::<Vec<_>>();
                    impact_nodes.sort_by(|a, b| {
                        a.file_path
                            .cmp(&b.file_path)
                            .then_with(|| a.start_line.cmp(&b.start_line))
                            .then_with(|| a.name.cmp(&b.name))
                    });
                    for n in impact_nodes.into_iter().take(limit) {
                        lines.push(format!("- {}", format_node(&n)));
                    }
                }
                Ok(text_result(lines.join("\n")))
            }
            "codegraph_paths" => {
                let from = required_str(args, "from")?;
                let to = required_str(args, "to")?;
                let depth = clamp(
                    args.get("depth").and_then(Value::as_i64).unwrap_or(4),
                    1,
                    10,
                ) as usize;
                let limit = clamp(
                    args.get("limit").and_then(Value::as_i64).unwrap_or(5),
                    1,
                    50,
                ) as usize;
                let from_node = find_matching_nodes(&cg, from)?.into_iter().next();
                let to_node = find_matching_nodes(&cg, to)?.into_iter().next();
                let (Some(from_node), Some(to_node)) = (from_node, to_node) else {
                    return Ok(text_result(format!(
                        "Could not resolve path endpoints: {from} -> {to}"
                    )));
                };
                let paths = cg.find_paths(&from_node.id, &to_node.id, depth, limit)?;
                Ok(text_result(format_paths(from, to, &paths)))
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
                ) as usize;
                let report = cg.build_explore_report(query, max_files)?;
                Ok(text_result(format_explore_report(&report, 35_000)))
            }
            "codegraph_status" => {
                let stats = cg.stats()?;
                Ok(text_result(format!(
                    "**Files indexed:** {}\n**Nodes:** {}\n**Edges:** {}\n**Last indexed at:** {}\n**Stale files:** {}",
                    stats.file_count,
                    stats.node_count,
                    stats.edge_count,
                    format_optional_timestamp_ms(stats.last_indexed_at),
                    stats.stale_file_count
                )))
            }
            "codegraph_files" => {
                let format = args
                    .get("format")
                    .and_then(Value::as_str)
                    .unwrap_or("tree")
                    .parse::<FileListFormat>()
                    .map_err(|_| {
                        anyhow!("codegraph_files format must be grouped, flat, or tree")
                    })?;
                let report = cg.list_files(FileListOptions {
                    format,
                    path_filter: args.get("path").and_then(Value::as_str).map(str::to_string),
                    pattern: args
                        .get("pattern")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                    include_metadata: args
                        .get("includeMetadata")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                    max_depth: args
                        .get("maxDepth")
                        .and_then(Value::as_i64)
                        .map(|depth| clamp(depth, 1, 20) as usize),
                })?;
                Ok(text_result(format_file_report(&report)))
            }
            "codegraph_affected" => {
                let files = required_string_array(args, "files")?;
                Ok(text_result(serde_json::to_string_pretty(
                    &cg.build_affected_report(&files)?,
                )?))
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
            json!({"task": {"type":"string"}, "maxNodes": {"type":"number"}, "includeCode": {"type":"boolean"}, "format": {"type":"string", "enum":["text", "json"]}, "projectPath": {"type":"string"}}),
            vec!["task"]
        ),
        tool(
            "codegraph_callers",
            "Find all functions/methods that call a specific symbol.",
            json!({"symbol": {"type":"string"}, "depth": {"type":"number"}, "limit": {"type":"number"}, "projectPath": {"type":"string"}}),
            vec!["symbol"]
        ),
        tool(
            "codegraph_callees",
            "Find all functions/methods that a specific symbol calls.",
            json!({"symbol": {"type":"string"}, "depth": {"type":"number"}, "limit": {"type":"number"}, "projectPath": {"type":"string"}}),
            vec!["symbol"]
        ),
        tool(
            "codegraph_impact",
            "Analyze the impact radius of changing a symbol.",
            json!({"symbol": {"type":"string"}, "depth": {"type":"number"}, "limit": {"type":"number"}, "projectPath": {"type":"string"}}),
            vec!["symbol"]
        ),
        tool(
            "codegraph_paths",
            "Find bounded dependency/call paths between two symbols.",
            json!({"from": {"type":"string"}, "to": {"type":"string"}, "depth": {"type":"number"}, "limit": {"type":"number"}, "projectPath": {"type":"string"}}),
            vec!["from", "to"]
        ),
        tool(
            "codegraph_node",
            "Get detailed information about a specific code symbol.",
            json!({"symbol": {"type":"string"}, "includeCode": {"type":"boolean"}, "projectPath": {"type":"string"}}),
            vec!["symbol"]
        ),
        tool(
            "codegraph_explore",
            "Deep exploration tool for a topic. Returns grouped source sections, relationship map, additional relevant files, and truncation notices. Budget guidance: small projects usually need 1-2 calls; medium projects need a few targeted calls; large projects should use narrow symbol/file queries.",
            json!({"query": {"type":"string"}, "maxFiles": {"type":"number", "minimum": 1, "maximum": 20}, "projectPath": {"type":"string"}}),
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
        tool(
            "codegraph_affected",
            "Return affected test candidates for changed files.",
            json!({"files": {"type":"array", "items": {"type":"string"}}, "projectPath": {"type":"string"}}),
            vec!["files"]
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

fn required_string_array(args: &Value, key: &str) -> Result<Vec<String>> {
    let values = args
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("{key} must be an array of strings"))?;
    let mut out = Vec::new();
    for value in values {
        let Some(item) = value.as_str().filter(|s| !s.is_empty()) else {
            return Err(anyhow!("{key} must be an array of non-empty strings"));
        };
        out.push(item.to_string());
    }
    Ok(out)
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
        lines.push(format!(
            "- depth {} {} via {}",
            edge.depth,
            format_node(&edge.node),
            edge.edge.kind
        ));
    }
    lines.join("\n")
}

fn format_paths(from: &str, to: &str, paths: &[crate::types::GraphPath]) -> String {
    if paths.is_empty() {
        return format!("No paths found from {from} to {to}");
    }
    let mut lines = vec![format!("## Paths: {from} -> {to}")];
    for (idx, path) in paths.iter().enumerate() {
        lines.push(format!("Path {}:", idx + 1));
        lines.push(
            path.nodes
                .iter()
                .map(format_node)
                .collect::<Vec<_>>()
                .join("\n  -> "),
        );
    }
    lines.join("\n")
}

fn format_file_report(report: &FileListReport) -> String {
    if report.total_files == 0 {
        return "No indexed files matched.".to_string();
    }
    match report.format.as_str() {
        "flat" => report
            .files
            .iter()
            .map(format_file_entry)
            .collect::<Vec<_>>()
            .join("\n"),
        "grouped" => report
            .groups
            .iter()
            .map(|group| {
                let mut lines = vec![format!("{}: {}", group.language, group.count)];
                for file in &group.files {
                    lines.push(format!("  {}", format_file_entry(file)));
                }
                lines.join("\n")
            })
            .collect::<Vec<_>>()
            .join("\n"),
        _ => {
            let mut lines = Vec::new();
            for entry in &report.tree {
                push_tree_entry(entry, 0, &mut lines);
            }
            lines.join("\n")
        }
    }
}

fn format_file_entry(file: &crate::types::FileListEntry) -> String {
    let mut out = format!(
        "{} ({}, {} symbols)",
        file.path, file.language, file.node_count
    );
    if let Some(size) = file.size {
        out.push_str(&format!(", {size} bytes"));
    }
    out
}

fn push_tree_entry(entry: &crate::types::FileTreeEntry, depth: usize, lines: &mut Vec<String>) {
    let indent = "  ".repeat(depth);
    if entry.kind == "dir" {
        lines.push(format!("{indent}{}/", entry.name));
        for child in &entry.children {
            push_tree_entry(child, depth + 1, lines);
        }
    } else {
        let mut line = format!(
            "{indent}{} ({}, {} symbols)",
            entry.name,
            entry
                .language
                .map(|lang| lang.as_str())
                .unwrap_or("unknown"),
            entry.node_count.unwrap_or_default()
        );
        if let Some(size) = entry.size {
            line.push_str(&format!(", {size} bytes"));
        }
        lines.push(line);
    }
}

fn format_explore_report(report: &crate::types::ExploreReport, max_chars: usize) -> String {
    let mut out = format!(
        "## Explore: {}\n\nBudget: {}\n\n",
        report.query, report.budget_guidance
    );

    if report.source_files.is_empty() {
        out.push_str("No matching source sections found.\n");
    } else {
        out.push_str("## Source Sections\n");
        for file in &report.source_files {
            out.push_str(&format!("\n### {} ({})\n", file.path, file.language));
            for section in &file.sections {
                out.push_str(&format!(
                    "- `{}` `{}` lines {}-{}: {}\n\n```{}\n",
                    section.kind,
                    section.symbol,
                    section.start_line,
                    section.end_line,
                    section.reason,
                    file.language.as_str()
                ));
                out.push_str(&section.code);
                if !section.code.ends_with('\n') {
                    out.push('\n');
                }
                out.push_str("```\n");
                if section.truncated {
                    out.push_str("[section truncated]\n");
                }
            }
        }
    }

    if !report.relationships.is_empty() {
        out.push_str("\n## Relationship Map\n");
        for relationship in &report.relationships {
            out.push_str(&format!(
                "- {} `{}` --{}--> `{}` ({})\n",
                relationship.direction,
                relationship.source,
                relationship.kind,
                relationship.target,
                relationship.file_path
            ));
        }
    }

    if !report.additional_files.is_empty() {
        out.push_str("\n## Additional Relevant Files\n");
        for file in &report.additional_files {
            out.push_str(&format!("- `{file}`\n"));
        }
    }

    if !report.warnings.is_empty() {
        out.push_str("\n## Warnings\n");
        for warning in &report.warnings {
            out.push_str(&format!("- {warning}\n"));
        }
    }

    if report.truncated {
        out.push_str("\n[truncated]");
        if let Some(reason) = &report.truncated_reason {
            out.push(' ');
            out.push_str(reason);
        }
    }

    if out.chars().count() > max_chars {
        let mut bounded = out.chars().take(max_chars).collect::<String>();
        bounded.push_str("\n\n[truncated] MCP response exceeded text budget.");
        bounded
    } else {
        out
    }
}

fn text_result(text: String) -> Value {
    json!({ "content": [{ "type": "text", "text": text }] })
}

fn format_optional_timestamp_ms(value: Option<i64>) -> String {
    value
        .map(|ms| ms.to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

fn error_response(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}
