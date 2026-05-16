use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use codegraph::types::{FileRecord, Language, SearchOptions};
use codegraph::{find_nearest_codegraph_root, is_initialized, CodeGraph};
use serde_json::json;
use std::collections::BTreeSet;
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
        Command::Context { task, path } => {
            let root = resolve_root(path)?;
            let cg = CodeGraph::open(root)?;
            println!("{}", cg.build_context(&task, 20, true)?);
        }
        Command::Affected { files, path, json } => {
            let root = resolve_root(path)?;
            let cg = CodeGraph::open(root)?;
            let indexed_files = cg.get_all_files()?;
            let mut affected = BTreeSet::new();
            let mut debug = Vec::new();
            for file in &files {
                if is_test_file(file) {
                    affected.insert(file.clone());
                    debug.push(json!({
                        "changedFile": file,
                        "reason": "changed file is a test file",
                        "matchedTests": [file],
                    }));
                    continue;
                }
                let mut matched = BTreeSet::new();
                for dep in cg.get_file_dependents(file)? {
                    if is_test_file(&dep) {
                        matched.insert(dep.clone());
                        affected.insert(dep);
                    }
                }
                let moonbit_tests = moonbit_same_package_tests(file, &indexed_files);
                for test in moonbit_tests {
                    matched.insert(test.clone());
                    affected.insert(test);
                }
                debug.push(json!({
                    "changedFile": file,
                    "reason": if matched.is_empty() {
                        "no import-dependent tests or MoonBit same-package tests found"
                    } else {
                        "matched import-dependent tests and/or MoonBit same-package tests"
                    },
                    "matchedTests": matched.into_iter().collect::<Vec<_>>(),
                }));
            }
            let affected: Vec<String> = affected.into_iter().collect();
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "changedFiles": files,
                        "affectedTests": affected,
                        "debug": debug,
                    }))?
                );
            } else {
                for f in affected {
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

fn is_test_file(file: &str) -> bool {
    let basename = file.rsplit('/').next().unwrap_or(file);
    file.ends_with(".mbt.md")
        || basename.ends_with("_test.mbt")
        || basename.ends_with("_wbtest.mbt")
        || file.contains("/__tests__/")
        || file.contains("/test/")
        || file.contains("/tests/")
        || file.contains("/e2e/")
        || file.contains("/spec/")
        || file.contains(".test.")
        || file.contains(".spec.")
}

fn moonbit_same_package_tests(file: &str, indexed_files: &[FileRecord]) -> Vec<String> {
    if is_test_file(file) || !is_moonbit_source_file(file) {
        return Vec::new();
    }
    let Some(package_dir) = moonbit_package_dir(file, indexed_files) else {
        return Vec::new();
    };
    indexed_files
        .iter()
        .filter(|record| record.language == Language::MoonBit)
        .filter(|record| is_test_file(&record.path))
        .filter(|record| {
            moonbit_package_dir(&record.path, indexed_files).as_deref() == Some(&package_dir)
        })
        .map(|record| record.path.clone())
        .collect()
}

fn is_moonbit_source_file(file: &str) -> bool {
    file.ends_with(".mbt") || file.ends_with(".mbti") || file.ends_with(".mbt.md")
}

fn moonbit_package_dir(file: &str, indexed_files: &[FileRecord]) -> Option<String> {
    let mut best: Option<&str> = None;
    for record in indexed_files {
        if !record.path.ends_with("moon.pkg.json") && !record.path.ends_with("moon.pkg") {
            continue;
        }
        let dir = record
            .path
            .rsplit_once('/')
            .map(|(dir, _)| dir)
            .unwrap_or("");
        if (dir.is_empty() || file == dir || file.starts_with(&format!("{dir}/")))
            && best
                .map(|current| dir.len() > current.len())
                .unwrap_or(true)
        {
            best = Some(dir);
        }
    }
    best.map(str::to_string)
}
