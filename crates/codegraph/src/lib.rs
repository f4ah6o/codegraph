pub mod config;
pub mod db;
pub mod extraction;
pub mod graph;
pub mod mcp;
pub mod types;

use anyhow::{anyhow, Context, Result};
use config::{load_config, save_config, CodeGraphConfig};
use db::Database;
use extraction::{detect_language, extract_from_source, should_include_file};
use graph::{GraphTraverser, Subgraph};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use types::{FileRecord, GraphStats, IndexResult, Node, NodeEdge, SearchOptions, SearchResult};

pub const CODEGRAPH_DIR: &str = ".codegraph";
pub const DATABASE_FILE: &str = "codegraph.db";

pub struct CodeGraph {
    root: PathBuf,
    config: CodeGraphConfig,
    db: Database,
}

impl CodeGraph {
    pub fn init(root: impl AsRef<Path>) -> Result<Self> {
        let root = root
            .as_ref()
            .canonicalize()
            .unwrap_or_else(|_| root.as_ref().to_path_buf());
        let dir = root.join(CODEGRAPH_DIR);
        if dir.exists() {
            return Err(anyhow!(
                "CodeGraph already initialized in {}",
                root.display()
            ));
        }
        fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
        let config = CodeGraphConfig::default_for_root(".");
        save_config(&root, &config)?;
        let db = Database::initialize(dir.join(DATABASE_FILE))?;
        Ok(Self { root, config, db })
    }

    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        let root = find_nearest_codegraph_root(root.as_ref())
            .ok_or_else(|| anyhow!("CodeGraph not initialized in {}", root.as_ref().display()))?;
        let config = load_config(&root)?;
        let db = Database::open(root.join(CODEGRAPH_DIR).join(DATABASE_FILE))?;
        Ok(Self { root, config, db })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn index_all(&mut self) -> Result<IndexResult> {
        let start = std::time::Instant::now();
        self.db.clear_all()?;
        let files = self.scan_files()?;
        let mut result = IndexResult::default();

        for path in files {
            let full = self.root.join(&path);
            let content = match fs::read_to_string(&full) {
                Ok(content) => content,
                Err(err) => {
                    result.files_errored += 1;
                    result.errors.push(format!("{}: {}", path.display(), err));
                    continue;
                }
            };
            let lang = detect_language(&path, &content);
            if lang.is_unknown() {
                result.files_skipped += 1;
                continue;
            }
            let extraction = extract_from_source(&path, &content, lang);
            let hash = content_hash(&content);
            let metadata = fs::metadata(&full)?;
            self.db.insert_file(&FileRecord {
                path: path.to_string_lossy().replace('\\', "/"),
                content_hash: hash,
                language: lang,
                size: metadata.len(),
                modified_at: metadata
                    .modified()
                    .ok()
                    .and_then(system_time_ms)
                    .unwrap_or_default(),
                indexed_at: now_ms(),
                node_count: extraction.nodes.len() as i64,
            })?;
            self.db.insert_nodes(&extraction.nodes)?;
            self.db.insert_edges(&extraction.edges)?;
            self.db
                .insert_unresolved_refs(&extraction.unresolved_references)?;
            result.files_indexed += 1;
            result.nodes_created += extraction.nodes.len() as i64;
            result.edges_created += extraction.edges.len() as i64;
        }

        self.db.resolve_references_by_name()?;
        result.edges_created = self.db.edge_count()?;
        result.success = result.files_errored == 0;
        result.duration_ms = start.elapsed().as_millis() as i64;
        Ok(result)
    }

    pub fn sync(&mut self) -> Result<IndexResult> {
        self.index_all()
    }

    pub fn stats(&self) -> Result<GraphStats> {
        self.db.stats()
    }

    pub fn search_nodes(&self, query: &str, options: SearchOptions) -> Result<Vec<SearchResult>> {
        self.db.search_nodes(query, options)
    }

    pub fn get_node(&self, id: &str) -> Result<Option<Node>> {
        self.db.get_node(id)
    }

    pub fn get_callers(&self, node_id: &str, max_depth: usize) -> Result<Vec<NodeEdge>> {
        GraphTraverser::new(&self.db).get_callers(node_id, max_depth)
    }

    pub fn get_callees(&self, node_id: &str, max_depth: usize) -> Result<Vec<NodeEdge>> {
        GraphTraverser::new(&self.db).get_callees(node_id, max_depth)
    }

    pub fn get_impact_radius(&self, node_id: &str, max_depth: usize) -> Result<Subgraph> {
        GraphTraverser::new(&self.db).get_impact_radius(node_id, max_depth)
    }

    pub fn get_file_dependents(&self, file_path: &str) -> Result<Vec<String>> {
        self.db.get_file_dependents(file_path)
    }

    pub fn get_all_files(&self) -> Result<Vec<FileRecord>> {
        self.db.get_all_files()
    }

    pub fn build_context(&self, task: &str, max_nodes: i64, include_code: bool) -> Result<String> {
        let results = self.find_context_nodes(task, max_nodes)?;
        let mut out = format!("## Context: {task}\n\n");
        if results.is_empty() {
            out.push_str("No matching symbols or files were found.\n\n");
            out.push_str("Try a concrete symbol name, file name, package/module name, or a shorter code term. ");
            out.push_str("For candidate discovery, run `cgz query --json <term>`.\n");
            return Ok(out);
        }
        for result in results {
            let n = result.node;
            out.push_str(&format!(
                "- `{}` `{}` at `{}:{}`",
                n.kind, n.name, n.file_path, n.start_line
            ));
            if let Some(sig) = n.signature.as_deref() {
                out.push_str(&format!(" — `{}`", sig.replace('\n', " ")));
            }
            out.push('\n');
            if include_code {
                if let Ok(code) = self.read_node_source(&n) {
                    out.push_str("\n```");
                    out.push_str(n.language.as_str());
                    out.push('\n');
                    out.push_str(&code);
                    if !code.ends_with('\n') {
                        out.push('\n');
                    }
                    out.push_str("```\n\n");
                }
            }
        }
        Ok(out)
    }

    fn find_context_nodes(&self, task: &str, max_nodes: i64) -> Result<Vec<SearchResult>> {
        let limit = max_nodes.max(1);
        let mut out = Vec::new();
        let mut seen = BTreeSet::new();

        for term in context_search_terms(task) {
            if out.len() >= limit as usize {
                break;
            }
            let remaining = limit - out.len() as i64;
            let results = self.search_nodes(
                &term,
                SearchOptions {
                    limit: remaining,
                    ..Default::default()
                },
            )?;
            for result in results {
                if seen.insert(result.node.id.clone()) {
                    out.push(result);
                    if out.len() >= limit as usize {
                        break;
                    }
                }
            }
        }

        Ok(out)
    }

    pub fn read_node_source(&self, node: &Node) -> Result<String> {
        let full = self.root.join(&node.file_path);
        let text =
            fs::read_to_string(&full).with_context(|| format!("reading {}", full.display()))?;
        let lines: Vec<&str> = text.lines().collect();
        let start = (node.start_line.saturating_sub(1) as usize).min(lines.len());
        let end = (node.end_line.max(node.start_line) as usize).min(lines.len());
        Ok(lines[start..end].join("\n"))
    }

    pub fn close(self) {}

    fn scan_files(&self) -> Result<Vec<PathBuf>> {
        let mut out = Vec::new();
        let walker = ignore::WalkBuilder::new(&self.root)
            .hidden(false)
            .git_ignore(true)
            .git_global(true)
            .git_exclude(true)
            .build();
        for entry in walker {
            let entry = entry?;
            if !entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
                continue;
            }
            let rel = entry
                .path()
                .strip_prefix(&self.root)
                .unwrap_or(entry.path())
                .to_path_buf();
            if rel.components().any(|c| c.as_os_str() == CODEGRAPH_DIR) {
                continue;
            }
            if should_include_file(&rel, &self.config) {
                out.push(rel);
            }
        }
        out.sort();
        Ok(out)
    }
}

fn context_search_terms(task: &str) -> Vec<String> {
    let mut terms = Vec::new();
    let mut seen = BTreeSet::new();
    push_context_term(task.trim(), &mut terms, &mut seen);

    for raw in task.split(|c: char| {
        !(c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '/' || c == '.' || c == ':')
    }) {
        let term = raw.trim_matches(|c: char| {
            !(c.is_ascii_alphanumeric() || c == '_' || c == '/' || c == '.' || c == ':')
        });
        if is_useful_context_term(term) {
            push_context_term(term, &mut terms, &mut seen);
        }
    }

    terms
}

fn push_context_term(term: &str, terms: &mut Vec<String>, seen: &mut BTreeSet<String>) {
    if term.is_empty() {
        return;
    }
    let key = term.to_ascii_lowercase();
    if seen.insert(key) {
        terms.push(term.to_string());
    }
}

fn is_useful_context_term(term: &str) -> bool {
    if term.len() < 3 {
        return false;
    }
    if CONTEXT_STOP_WORDS.contains(&term.to_ascii_lowercase().as_str()) {
        return false;
    }
    term.contains('_')
        || term.contains('/')
        || term.contains('.')
        || term.contains(':')
        || term.chars().any(|c| c.is_ascii_digit())
        || term.len() >= 5
}

const CONTEXT_STOP_WORDS: &[&str] = &[
    "about",
    "after",
    "before",
    "build",
    "change",
    "check",
    "code",
    "context",
    "debug",
    "error",
    "feature",
    "files",
    "fix",
    "from",
    "handle",
    "implement",
    "invalid",
    "issue",
    "order",
    "query",
    "return",
    "should",
    "task",
    "test",
    "tests",
    "update",
    "valid",
    "validation",
    "when",
    "where",
    "with",
];

pub fn is_initialized(root: impl AsRef<Path>) -> bool {
    root.as_ref()
        .join(CODEGRAPH_DIR)
        .join(DATABASE_FILE)
        .exists()
}

pub fn find_nearest_codegraph_root(start: impl AsRef<Path>) -> Option<PathBuf> {
    let mut cur = start
        .as_ref()
        .canonicalize()
        .unwrap_or_else(|_| start.as_ref().to_path_buf());
    if cur.is_file() {
        cur.pop();
    }
    loop {
        if is_initialized(&cur) {
            return Some(cur);
        }
        if !cur.pop() {
            return None;
        }
    }
}

fn content_hash(content: &str) -> String {
    let mut h = Sha256::new();
    h.update(content.as_bytes());
    format!("{:x}", h.finalize())
}

fn now_ms() -> i64 {
    system_time_ms(std::time::SystemTime::now()).unwrap_or_default()
}

fn system_time_ms(t: std::time::SystemTime) -> Option<i64> {
    t.duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_millis() as i64)
}
