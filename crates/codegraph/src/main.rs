use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use codegraph::installer::InstallOptions;
use codegraph::types::{FileListFormat, FileListOptions, FileListReport, SearchOptions};
use codegraph::watcher::{run_watcher, WatcherConfig};
use codegraph::{find_nearest_codegraph_root, is_initialized, CodeGraph};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "cgz")]
#[command(about = "Code intelligence and knowledge graph for any codebase")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    Init {
        path: Option<PathBuf>,
        #[arg(short, long)]
        index: bool,
    },
    Uninit {
        path: Option<PathBuf>,
        #[arg(short, long)]
        force: bool,
    },
    Index {
        path: Option<PathBuf>,
        #[arg(short, long)]
        force: bool,
        #[arg(short, long)]
        quiet: bool,
    },
    Sync {
        path: Option<PathBuf>,
        #[arg(short, long)]
        quiet: bool,
    },
    Status {
        path: Option<PathBuf>,
        #[arg(short, long)]
        json: bool,
    },
    Query {
        search: String,
        #[arg(short, long)]
        path: Option<PathBuf>,
        #[arg(short, long, default_value_t = 10)]
        limit: i64,
        #[arg(short, long)]
        json: bool,
    },
    Files {
        #[arg(short, long)]
        path: Option<PathBuf>,
        #[arg(long, default_value = "grouped")]
        format: String,
        #[arg(long)]
        filter_path: Option<String>,
        #[arg(long)]
        pattern: Option<String>,
        #[arg(long)]
        include_metadata: bool,
        #[arg(long)]
        max_depth: Option<usize>,
        #[arg(short, long)]
        json: bool,
    },
    Context {
        task: String,
        #[arg(short, long)]
        path: Option<PathBuf>,
        #[arg(short, long)]
        json: bool,
    },
    Affected {
        files: Vec<String>,
        #[arg(short, long)]
        path: Option<PathBuf>,
        #[arg(short, long)]
        json: bool,
    },
    Callers {
        symbol: String,
        #[arg(short, long)]
        path: Option<PathBuf>,
        #[arg(short, long, default_value_t = 2)]
        depth: usize,
        #[arg(short, long, default_value_t = 20)]
        limit: usize,
        #[arg(short, long)]
        json: bool,
    },
    Callees {
        symbol: String,
        #[arg(short, long)]
        path: Option<PathBuf>,
        #[arg(short, long, default_value_t = 2)]
        depth: usize,
        #[arg(short, long, default_value_t = 20)]
        limit: usize,
        #[arg(short, long)]
        json: bool,
    },
    Impact {
        symbol: String,
        #[arg(short, long)]
        path: Option<PathBuf>,
        #[arg(short, long, default_value_t = 2)]
        depth: usize,
        #[arg(short, long, default_value_t = 50)]
        limit: usize,
        #[arg(short, long)]
        json: bool,
    },
    Paths {
        from: String,
        to: String,
        #[arg(short, long)]
        path: Option<PathBuf>,
        #[arg(short, long, default_value_t = 4)]
        depth: usize,
        #[arg(short, long, default_value_t = 5)]
        limit: usize,
        #[arg(short, long)]
        json: bool,
    },
    Serve {
        #[arg(long)]
        mcp: bool,
        #[arg(short, long)]
        path: Option<PathBuf>,
    },
    Unlock {
        path: Option<PathBuf>,
    },
    Watch {
        path: Option<PathBuf>,
        #[arg(short, long, default_value_t = 300)]
        debounce: u64,
    },
    Skills,
    Install {
        #[arg(long, conflicts_with = "local")]
        global: bool,
        #[arg(long)]
        local: bool,
        #[arg(short, long)]
        yes: bool,
        #[arg(long)]
        no_init: bool,
        #[arg(long)]
        allow_permissions: bool,
        #[arg(short, long)]
        path: Option<PathBuf>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let Some(command) = cli.command else {
        println!("Run `cgz install --local` to configure Claude, or `cgz init -i` to initialize a project.");
        return Ok(());
    };

    match command {
        Command::Init { path, index } => {
            let root = path.unwrap_or(std::env::current_dir()?);
            let mut cg = CodeGraph::init(&root)?;
            println!("Initialized in {}", cg.root().display());
            if index {
                let result = cg.index_all()?;
                print_index_result(&result);
            } else {
                println!("Run `cgz index` to index the project");
            }
        }
        Command::Uninit { path, force } => {
            let root = resolve_root(path)?;
            if !force {
                eprintln!(
                    "Refusing to remove {} without --force",
                    root.join(".codegraph").display()
                );
                std::process::exit(1);
            }
            std::fs::remove_dir_all(root.join(".codegraph"))?;
            println!("Removed CodeGraph data");
        }
        Command::Index { path, quiet, .. } => {
            let root = resolve_root(path)?;
            let mut cg = CodeGraph::open(root)?;
            let result = cg.index_all()?;
            if !quiet {
                print_index_result(&result);
            }
            if !result.success {
                std::process::exit(1);
            }
        }
        Command::Sync { path, quiet } => {
            let root = resolve_root(path)?;
            let mut cg = CodeGraph::open(root)?;
            let result = cg.sync()?;
            if !quiet {
                print_index_result(&result);
            }
        }
        Command::Status { path, json } => {
            let root = path.unwrap_or(std::env::current_dir()?);
            if !is_initialized(&root) && find_nearest_codegraph_root(&root).is_none() {
                if json {
                    println!(
                        "{}",
                        serde_json::json!({ "initialized": false, "projectPath": root })
                    );
                } else {
                    println!("CodeGraph not initialized in {}", root.display());
                }
                return Ok(());
            }
            let cg = CodeGraph::open(root)?;
            let stats = cg.stats()?;
            if json {
                println!("{}", serde_json::to_string_pretty(&stats)?);
            } else {
                println!("CodeGraph Status");
                println!("Files: {}", stats.file_count);
                println!("Nodes: {}", stats.node_count);
                println!("Edges: {}", stats.edge_count);
                println!("DB Size: {} bytes", stats.db_size_bytes);
                println!(
                    "Last Indexed At: {}",
                    format_optional_timestamp_ms(stats.last_indexed_at)
                );
                println!(
                    "Oldest Indexed At: {}",
                    format_optional_timestamp_ms(stats.oldest_indexed_at)
                );
                println!(
                    "Newest Modified At: {}",
                    format_optional_timestamp_ms(stats.newest_modified_at)
                );
                println!("Stale Files: {}", stats.stale_file_count);
                println!("Files by Language:");
                for (lang, count) in stats.files_by_language {
                    println!("  {lang:<15} {count}");
                }
            }
        }
        Command::Query {
            search,
            path,
            limit,
            json,
        } => {
            let root = resolve_root(path)?;
            let cg = CodeGraph::open(root)?;
            let results = cg.search_nodes(
                &search,
                SearchOptions {
                    limit,
                    ..Default::default()
                },
            )?;
            if json {
                println!("{}", serde_json::to_string_pretty(&results)?);
            } else if results.is_empty() {
                println!("No results found for \"{}\"", search);
            } else {
                for r in results {
                    println!(
                        "{} {} {}:{}",
                        r.node.kind, r.node.name, r.node.file_path, r.node.start_line
                    );
                }
            }
        }
        Command::Files {
            path,
            format,
            filter_path,
            pattern,
            include_metadata,
            max_depth,
            json,
        } => {
            let root = resolve_root(path)?;
            let cg = CodeGraph::open(root)?;
            let legacy_json = json
                && format == "grouped"
                && filter_path.is_none()
                && pattern.is_none()
                && !include_metadata
                && max_depth.is_none();
            let format = format
                .parse::<FileListFormat>()
                .map_err(|_| anyhow!("files --format must be grouped, flat, or tree"))?;
            let report = cg.list_files(FileListOptions {
                format,
                path_filter: filter_path,
                pattern,
                include_metadata,
                max_depth,
            })?;
            if legacy_json {
                let stats = cg.stats()?;
                println!(
                    "{}",
                    serde_json::to_string_pretty(&stats.files_by_language)?
                );
            } else if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!("{}", format_file_report(&report));
            }
        }
        Command::Context { task, path, json } => {
            let root = resolve_root(path)?;
            let cg = CodeGraph::open(root)?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&cg.build_context_report(&task, 20, true)?)?
                );
            } else {
                println!("{}", cg.build_context(&task, 20, true)?);
            }
        }
        Command::Affected { files, path, json } => {
            let root = resolve_root(path)?;
            let cg = CodeGraph::open(root)?;
            let report = cg.build_affected_report(&files)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                for f in report.affected_tests {
                    println!("{f}");
                }
            }
        }
        Command::Callers {
            symbol,
            path,
            depth,
            limit,
            json,
        } => {
            let root = resolve_root(path)?;
            let cg = CodeGraph::open(root)?;
            let nodes = find_cli_nodes(&cg, &symbol)?;
            let mut out = Vec::new();
            for node in nodes {
                out.extend(cg.get_callers(&node.id, depth.min(10))?);
            }
            out.truncate(limit.min(200));
            if json {
                println!("{}", serde_json::to_string_pretty(&out)?);
            } else {
                print_node_edges(&format!("Callers of {symbol}"), &out);
            }
        }
        Command::Callees {
            symbol,
            path,
            depth,
            limit,
            json,
        } => {
            let root = resolve_root(path)?;
            let cg = CodeGraph::open(root)?;
            let nodes = find_cli_nodes(&cg, &symbol)?;
            let mut out = Vec::new();
            for node in nodes {
                out.extend(cg.get_callees(&node.id, depth.min(10))?);
            }
            out.truncate(limit.min(200));
            if json {
                println!("{}", serde_json::to_string_pretty(&out)?);
            } else {
                print_node_edges(&format!("Callees of {symbol}"), &out);
            }
        }
        Command::Impact {
            symbol,
            path,
            depth,
            limit,
            json,
        } => {
            let root = resolve_root(path)?;
            let cg = CodeGraph::open(root)?;
            let nodes = find_cli_nodes(&cg, &symbol)?;
            let mut impacts = Vec::new();
            for node in nodes {
                impacts.push(cg.get_impact_radius(&node.id, depth.min(10))?);
            }
            if json {
                println!("{}", serde_json::to_string_pretty(&impacts)?);
            } else {
                println!("Impact of {symbol}");
                let mut printed = 0usize;
                for impact in impacts {
                    let mut nodes = impact.nodes.into_values().collect::<Vec<_>>();
                    nodes.sort_by(|a, b| {
                        a.file_path
                            .cmp(&b.file_path)
                            .then_with(|| a.start_line.cmp(&b.start_line))
                            .then_with(|| a.name.cmp(&b.name))
                    });
                    for node in nodes {
                        if printed >= limit.min(200) {
                            return Ok(());
                        }
                        println!("- {}", format_cli_node(&node));
                        printed += 1;
                    }
                }
            }
        }
        Command::Paths {
            from,
            to,
            path,
            depth,
            limit,
            json,
        } => {
            let root = resolve_root(path)?;
            let cg = CodeGraph::open(root)?;
            let from_node = find_cli_nodes(&cg, &from)?.into_iter().next();
            let to_node = find_cli_nodes(&cg, &to)?.into_iter().next();
            let (Some(from_node), Some(to_node)) = (from_node, to_node) else {
                return Err(anyhow!("Could not resolve both path endpoints"));
            };
            let paths = cg.find_paths(&from_node.id, &to_node.id, depth.min(10), limit.min(50))?;
            if json {
                println!("{}", serde_json::to_string_pretty(&paths)?);
            } else if paths.is_empty() {
                println!("No paths found from {from} to {to}");
            } else {
                for (idx, path) in paths.iter().enumerate() {
                    println!("Path {}:", idx + 1);
                    println!("{}", format_path(path));
                }
            }
        }
        Command::Serve { mcp, path } => {
            if mcp {
                let mut server = codegraph::mcp::MCPServer::new(path);
                server.start()?;
                return Ok(());
            }
        }
        Command::Unlock { path } => {
            let root = resolve_root(path)?;
            let lock = root.join(".codegraph").join("codegraph.lock");
            if lock.exists() {
                std::fs::remove_file(lock)?;
            }
            println!("Unlocked");
        }
        Command::Watch { path, debounce } => {
            let root = path.unwrap_or(std::env::current_dir()?);
            run_watcher(
                root,
                WatcherConfig {
                    debounce_ms: debounce,
                },
            )?;
        }
        Command::Skills => {
            print!("{}", include_str!("../assets/cgz-skill.md"));
        }
        Command::Install {
            global,
            local,
            yes,
            no_init,
            allow_permissions,
            path,
        } => {
            let result = codegraph::installer::install(&InstallOptions {
                global,
                local,
                yes,
                no_init,
                allow_permissions,
                project_path: path,
                home_dir: None,
            })?;
            println!("Claude MCP config: {}", result.claude_json_path.display());
            if result.claude_json_changed {
                println!("  Added CodeGraph MCP server configuration");
            } else {
                println!("  CodeGraph MCP server configuration already up to date");
            }
            if let Some(settings_path) = result.settings_json_path.as_ref() {
                println!("Claude settings: {}", settings_path.display());
                if result.settings_json_changed {
                    println!("  Added CodeGraph MCP tool permissions");
                } else {
                    println!("  CodeGraph MCP tool permissions already up to date");
                }
            }
            println!("CLAUDE.md: {}", result.claude_md_path.display());
            if result.claude_md_changed {
                println!("  Added CodeGraph section to CLAUDE.md");
            } else {
                println!("  CLAUDE.md CodeGraph section already up to date");
            }
            if !result.init_message.is_empty() {
                println!("{}", result.init_message);
            }
        }
    }
    Ok(())
}

fn find_cli_nodes(cg: &CodeGraph, symbol: &str) -> Result<Vec<codegraph::types::Node>> {
    Ok(cg
        .search_nodes(
            symbol,
            SearchOptions {
                limit: 20,
                ..Default::default()
            },
        )?
        .into_iter()
        .map(|r| r.node)
        .collect())
}

fn print_node_edges(title: &str, edges: &[codegraph::types::NodeEdge]) {
    if edges.is_empty() {
        println!("No results found for {title}");
        return;
    }
    println!("{title}");
    for edge in edges {
        println!(
            "- depth {} {} via {}",
            edge.depth,
            format_cli_node(&edge.node),
            edge.edge.kind
        );
    }
}

fn format_cli_node(node: &codegraph::types::Node) -> String {
    format!(
        "{} {} {}:{}",
        node.kind, node.name, node.file_path, node.start_line
    )
}

fn format_path(path: &codegraph::types::GraphPath) -> String {
    path.nodes
        .iter()
        .map(format_cli_node)
        .collect::<Vec<_>>()
        .join("\n  -> ")
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
        "tree" => {
            let mut lines = Vec::new();
            for entry in &report.tree {
                push_tree_entry(entry, 0, &mut lines);
            }
            lines.join("\n")
        }
        _ => report
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
    }
}

fn format_file_entry(file: &codegraph::types::FileListEntry) -> String {
    let mut out = format!(
        "{} ({}, {} symbols)",
        file.path, file.language, file.node_count
    );
    if let Some(size) = file.size {
        out.push_str(&format!(", {size} bytes"));
    }
    out
}

fn push_tree_entry(entry: &codegraph::types::FileTreeEntry, depth: usize, lines: &mut Vec<String>) {
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

fn resolve_root(path: Option<PathBuf>) -> Result<PathBuf> {
    let start = path.unwrap_or(std::env::current_dir()?);
    find_nearest_codegraph_root(&start)
        .ok_or_else(|| anyhow!("CodeGraph not initialized in {}", start.display()))
}

fn print_index_result(result: &codegraph::types::IndexResult) {
    println!(
        "Indexed {} files, skipped {}, deleted {}, {} nodes, {} edges in {}ms",
        result.files_indexed,
        result.files_skipped,
        result.files_deleted,
        result.nodes_created,
        result.edges_created,
        result.duration_ms
    );
    if !result.errors.is_empty() {
        eprintln!("Errors:");
        for err in &result.errors {
            eprintln!("  {err}");
        }
    }
}

fn format_optional_timestamp_ms(value: Option<i64>) -> String {
    value
        .map(|ms| ms.to_string())
        .unwrap_or_else(|| "unknown".to_string())
}
