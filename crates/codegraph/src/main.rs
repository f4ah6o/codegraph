use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use codegraph::types::SearchOptions;
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
    Serve {
        #[arg(long)]
        mcp: bool,
        #[arg(short, long)]
        path: Option<PathBuf>,
    },
    Unlock {
        path: Option<PathBuf>,
    },
    Install,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command.unwrap_or(Command::Install) {
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
        Command::Files { path, json } => {
            let root = resolve_root(path)?;
            let cg = CodeGraph::open(root)?;
            let stats = cg.stats()?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&stats.files_by_language)?
                );
            } else {
                for (lang, count) in stats.files_by_language {
                    println!("{lang}: {count}");
                }
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
        Command::Install => {
            println!(
                "Rust CodeGraph installer is not implemented yet. Run `cgz init -i` in a project."
            );
        }
    }
    Ok(())
}

fn resolve_root(path: Option<PathBuf>) -> Result<PathBuf> {
    let start = path.unwrap_or(std::env::current_dir()?);
    find_nearest_codegraph_root(&start)
        .ok_or_else(|| anyhow!("CodeGraph not initialized in {}", start.display()))
}

fn print_index_result(result: &codegraph::types::IndexResult) {
    println!(
        "Indexed {} files, {} nodes, {} edges in {}ms",
        result.files_indexed, result.nodes_created, result.edges_created, result.duration_ms
    );
    if !result.errors.is_empty() {
        eprintln!("Errors:");
        for err in &result.errors {
            eprintln!("  {err}");
        }
    }
}
