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
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use types::{
    AffectedDebugEntry, AffectedMatchSources, AffectedReport, ContextFileSummary, ContextMatch,
    ContextReport, ContextSymbolSummary, FileRecord, GraphStats, IndexResult, Language, Node,
    NodeEdge, SearchOptions, SearchResult,
};

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

    pub fn build_affected_report(&self, files: &[String]) -> Result<AffectedReport> {
        let indexed_files = self.get_all_files()?;
        let moonbit_packages = MoonBitPackageGraph::from_root(&self.root, &indexed_files);
        let mut affected = BTreeSet::new();
        let mut debug = Vec::new();
        let mut warnings = Vec::new();

        for file in files {
            if is_test_file(file) {
                affected.insert(file.clone());
                debug.push(AffectedDebugEntry {
                    changed_file: file.clone(),
                    reason: "changed file is a test file".to_string(),
                    matched_tests: vec![file.clone()],
                    matched_by: AffectedMatchSources {
                        direct_test_input: vec![file.clone()],
                        import_dependents: Vec::new(),
                        moonbit_same_package: Vec::new(),
                        moonbit_package_dependents: Vec::new(),
                        rust_name_heuristic: Vec::new(),
                        rust_workspace_heuristic: Vec::new(),
                    },
                });
                continue;
            }

            let mut matched = BTreeSet::new();
            let mut import_dependents = BTreeSet::new();
            for dep in self.get_file_dependents(file)? {
                if is_test_file(&dep) {
                    import_dependents.insert(dep.clone());
                    matched.insert(dep.clone());
                    affected.insert(dep);
                }
            }

            let moonbit_tests: BTreeSet<String> = moonbit_same_package_tests(file, &indexed_files)
                .into_iter()
                .collect();
            for test in &moonbit_tests {
                matched.insert(test.clone());
                affected.insert(test.clone());
            }
            let moonbit_package_tests: BTreeSet<String> = moonbit_packages
                .dependent_package_tests(file)
                .into_iter()
                .collect();
            for test in &moonbit_package_tests {
                matched.insert(test.clone());
                affected.insert(test.clone());
            }
            let rust_tests: BTreeSet<String> = rust_name_heuristic_tests(file, &indexed_files)
                .into_iter()
                .collect();
            for test in &rust_tests {
                matched.insert(test.clone());
                affected.insert(test.clone());
            }
            let rust_workspace_tests: BTreeSet<String> =
                rust_workspace_heuristic_tests(&self.root, file, &indexed_files)
                    .into_iter()
                    .collect();
            for test in &rust_workspace_tests {
                matched.insert(test.clone());
                affected.insert(test.clone());
            }

            if matched.is_empty() {
                warnings.push(format!(
                    "{file}: no import-dependent tests, MoonBit same-package tests, MoonBit package-dependent tests, Rust name-heuristic tests, or Rust workspace tests found"
                ));
            }
            debug.push(AffectedDebugEntry {
                changed_file: file.clone(),
                reason: if matched.is_empty() {
                    "no import-dependent tests, MoonBit same-package tests, MoonBit package-dependent tests, Rust name-heuristic tests, or Rust workspace tests found".to_string()
                } else {
                    "matched import-dependent tests, MoonBit same-package tests, MoonBit package-dependent tests, Rust name-heuristic tests, and/or Rust workspace tests".to_string()
                },
                matched_tests: matched.into_iter().collect(),
                matched_by: AffectedMatchSources {
                    direct_test_input: Vec::new(),
                    import_dependents: import_dependents.into_iter().collect(),
                    moonbit_same_package: moonbit_tests.into_iter().collect(),
                    moonbit_package_dependents: moonbit_package_tests.into_iter().collect(),
                    rust_name_heuristic: rust_tests.into_iter().collect(),
                    rust_workspace_heuristic: rust_workspace_tests.into_iter().collect(),
                },
            });
        }

        Ok(AffectedReport {
            changed_files: files.to_vec(),
            affected_tests: affected.into_iter().collect(),
            debug,
            warnings,
        })
    }

    pub fn build_context(&self, task: &str, max_nodes: i64, include_code: bool) -> Result<String> {
        let report = self.build_context_report(task, max_nodes, include_code)?;
        let mut out = format!("## Context: {task}\n\n");
        if report.matches.is_empty() {
            for warning in &report.warnings {
                out.push_str(warning);
                out.push('\n');
            }
            return Ok(out);
        }

        for result in report.matches {
            let n = result.node;
            out.push_str(&format!(
                "- `{}` `{}` at `{}:{}`",
                n.kind, n.name, n.file_path, n.start_line
            ));
            if let Some(sig) = n.signature.as_deref() {
                out.push_str(&format!(" — `{}`", sig.replace('\n', " ")));
            }
            out.push('\n');
            if let Some(code) = result.code {
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
        Ok(out)
    }

    pub fn build_context_report(
        &self,
        task: &str,
        max_nodes: i64,
        include_code: bool,
    ) -> Result<ContextReport> {
        let query = task.trim().to_string();
        let search_terms = context_search_terms(task);
        let results = self.find_context_nodes(&search_terms, max_nodes)?;
        let mut matches = Vec::new();
        let mut files: BTreeMap<String, ContextFileSummary> = BTreeMap::new();
        let mut symbols = Vec::new();

        for (result, search_term) in results {
            let code = if include_code {
                self.read_node_source(&result.node).ok()
            } else {
                None
            };
            let file = files
                .entry(result.node.file_path.clone())
                .or_insert_with(|| ContextFileSummary {
                    path: result.node.file_path.clone(),
                    language: result.node.language,
                    match_count: 0,
                    symbols: Vec::new(),
                });
            file.match_count += 1;
            if !file.symbols.iter().any(|name| name == &result.node.name) {
                file.symbols.push(result.node.name.clone());
            }
            symbols.push(ContextSymbolSummary {
                name: result.node.name.clone(),
                kind: result.node.kind,
                file_path: result.node.file_path.clone(),
                start_line: result.node.start_line,
            });
            matches.push(ContextMatch {
                reason: context_match_reason(task, &search_term),
                search_term,
                score: result.score,
                node: result.node,
                code,
            });
        }

        let mut warnings = Vec::new();
        if matches.is_empty() {
            warnings.push("No matching symbols or files were found.".to_string());
            warnings.push(
                "Try a concrete symbol name, file name, package/module name, or a shorter code term. For candidate discovery, run `cgz query --json <term>`."
                    .to_string(),
            );
        }

        Ok(ContextReport {
            query,
            search_terms,
            matches,
            files: files.into_values().collect(),
            symbols,
            warnings,
        })
    }

    fn find_context_nodes(
        &self,
        search_terms: &[String],
        max_nodes: i64,
    ) -> Result<Vec<(SearchResult, String)>> {
        let limit = max_nodes.max(1);
        let mut out = Vec::new();
        let mut seen = BTreeSet::new();

        for term in search_terms {
            if out.len() >= limit as usize {
                break;
            }
            let remaining = limit - out.len() as i64;
            let results = self.search_nodes(
                term,
                SearchOptions {
                    limit: remaining,
                    ..Default::default()
                },
            )?;
            for result in results {
                if seen.insert(result.node.id.clone()) {
                    out.push((result, term.clone()));
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

fn context_match_reason(task: &str, search_term: &str) -> String {
    if task.trim().eq_ignore_ascii_case(search_term) {
        "matched the full context query".to_string()
    } else {
        format!("matched extracted task term `{search_term}`")
    }
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
        || term.chars().any(|c| c.is_ascii_uppercase())
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
    "how",
    "implement",
    "implemented",
    "invalid",
    "is",
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
    "what",
    "when",
    "where",
    "which",
    "who",
    "why",
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

#[derive(Debug, Default)]
struct MoonBitPackageGraph {
    package_by_dir: BTreeMap<String, MoonBitPackage>,
    reverse_imports: BTreeMap<String, BTreeSet<String>>,
}

#[derive(Debug)]
struct MoonBitPackage {
    name: String,
    imports: Vec<String>,
    tests: Vec<String>,
}

impl MoonBitPackageGraph {
    fn from_root(root: &Path, indexed_files: &[FileRecord]) -> Self {
        let module_name = moonbit_module_name(root, indexed_files);
        let mut package_by_dir = BTreeMap::new();

        for record in indexed_files {
            if !is_moonbit_package_file(&record.path) {
                continue;
            }
            let dir = parent_dir(&record.path);
            let source = fs::read_to_string(root.join(&record.path)).unwrap_or_default();
            let (name, imports) = parse_moonbit_package_metadata(&source);
            let package_name =
                name.unwrap_or_else(|| moonbit_package_name_from_dir(module_name.as_deref(), &dir));
            package_by_dir.insert(
                dir.clone(),
                MoonBitPackage {
                    name: package_name,
                    imports,
                    tests: Vec::new(),
                },
            );
        }

        let package_dirs: Vec<String> = package_by_dir.keys().cloned().collect();
        for record in indexed_files {
            if record.language != Language::MoonBit || !is_test_file(&record.path) {
                continue;
            }
            if let Some(package_dir) = moonbit_package_dir_from_dirs(&record.path, &package_dirs) {
                if let Some(package) = package_by_dir.get_mut(&package_dir) {
                    package.tests.push(record.path.clone());
                }
            }
        }

        let local_names: BTreeSet<String> = package_by_dir
            .values()
            .map(|package| package.name.clone())
            .collect();
        let mut reverse_imports: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for package in package_by_dir.values() {
            for import in &package.imports {
                if local_names.contains(import) {
                    reverse_imports
                        .entry(import.clone())
                        .or_default()
                        .insert(package.name.clone());
                }
            }
        }

        Self {
            package_by_dir,
            reverse_imports,
        }
    }

    fn dependent_package_tests(&self, file: &str) -> Vec<String> {
        if is_test_file(file) || !is_moonbit_source_file(file) {
            return Vec::new();
        }
        let Some(changed_package) = self.package_for_file(file) else {
            return Vec::new();
        };

        let mut pending: Vec<String> = self
            .reverse_imports
            .get(&changed_package.name)
            .map(|deps| deps.iter().cloned().collect())
            .unwrap_or_default();
        let mut dependent_names = BTreeSet::new();
        while let Some(package_name) = pending.pop() {
            if !dependent_names.insert(package_name.clone()) {
                continue;
            }
            if let Some(next) = self.reverse_imports.get(&package_name) {
                pending.extend(next.iter().cloned());
            }
        }

        self.package_by_dir
            .values()
            .filter(|package| dependent_names.contains(&package.name))
            .flat_map(|package| package.tests.clone())
            .collect()
    }

    fn package_for_file(&self, file: &str) -> Option<&MoonBitPackage> {
        let package_dir = moonbit_package_dir_from_dirs(file, self.package_by_dir.keys())?;
        self.package_by_dir.get(&package_dir)
    }
}

fn moonbit_module_name(root: &Path, indexed_files: &[FileRecord]) -> Option<String> {
    indexed_files
        .iter()
        .filter(|record| record.path.ends_with("moon.mod.json"))
        .min_by_key(|record| record.path.matches('/').count())
        .and_then(|record| fs::read_to_string(root.join(&record.path)).ok())
        .and_then(|source| {
            serde_json::from_str::<serde_json::Value>(&source)
                .ok()
                .and_then(|json| {
                    json.get("name")
                        .and_then(|value| value.as_str())
                        .map(str::to_string)
                })
        })
}

fn parse_moonbit_package_metadata(source: &str) -> (Option<String>, Vec<String>) {
    let Ok(json) = serde_json::from_str::<serde_json::Value>(source) else {
        return (None, Vec::new());
    };
    let name = json
        .get("name")
        .and_then(|value| value.as_str())
        .map(str::to_string);
    let mut imports = Vec::new();
    if let Some(value) = json.get("import").or_else(|| json.get("imports")) {
        collect_moonbit_imports(value, &mut imports);
    }
    (name, imports)
}

fn collect_moonbit_imports(value: &serde_json::Value, imports: &mut Vec<String>) {
    match value {
        serde_json::Value::String(import) => imports.push(import.clone()),
        serde_json::Value::Array(values) => {
            for value in values {
                collect_moonbit_imports(value, imports);
            }
        }
        serde_json::Value::Object(values) => {
            for (alias, value) in values {
                imports.push(value.as_str().unwrap_or(alias).to_string());
            }
        }
        _ => {}
    }
}

fn moonbit_package_name_from_dir(module_name: Option<&str>, dir: &str) -> String {
    match (module_name, dir.is_empty()) {
        (Some(module), true) => module.to_string(),
        (Some(module), false) => format!("{module}/{dir}"),
        (None, true) => "moonbit-package".to_string(),
        (None, false) => dir.to_string(),
    }
}

fn is_moonbit_source_file(file: &str) -> bool {
    file.ends_with(".mbt") || file.ends_with(".mbti") || file.ends_with(".mbt.md")
}

fn is_moonbit_package_file(file: &str) -> bool {
    file.ends_with("moon.pkg.json") || file.ends_with("moon.pkg")
}

fn moonbit_package_dir(file: &str, indexed_files: &[FileRecord]) -> Option<String> {
    let dirs: Vec<String> = indexed_files
        .iter()
        .filter(|record| is_moonbit_package_file(&record.path))
        .map(|record| parent_dir(&record.path))
        .collect();
    moonbit_package_dir_from_dirs(file, &dirs)
}

fn moonbit_package_dir_from_dirs<'a, I>(file: &str, dirs: I) -> Option<String>
where
    I: IntoIterator<Item = &'a String>,
{
    let mut best: Option<&str> = None;
    for dir in dirs {
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

fn parent_dir(file: &str) -> String {
    file.rsplit_once('/')
        .map(|(dir, _)| dir.to_string())
        .unwrap_or_default()
}

fn rust_name_heuristic_tests(file: &str, indexed_files: &[FileRecord]) -> Vec<String> {
    let Some(changed) = indexed_files.iter().find(|record| record.path == file) else {
        return Vec::new();
    };
    if changed.language != Language::Rust || is_test_file(file) {
        return Vec::new();
    }
    let Some(stem) = file
        .rsplit('/')
        .next()
        .and_then(|name| name.strip_suffix(".rs"))
    else {
        return Vec::new();
    };
    if stem.len() < 3 {
        return Vec::new();
    }
    indexed_files
        .iter()
        .filter(|record| record.language == Language::Rust)
        .filter(|record| is_test_file(&record.path))
        .filter(|record| rust_test_path_matches_stem(&record.path, stem))
        .map(|record| record.path.clone())
        .collect()
}

fn rust_test_path_matches_stem(test_path: &str, stem: &str) -> bool {
    test_path
        .rsplit('/')
        .next()
        .unwrap_or(test_path)
        .strip_suffix(".rs")
        .map(|name| {
            name == stem
                || name.ends_with(&format!("_{stem}"))
                || name.starts_with(&format!("{stem}_"))
                || name.contains(&format!("_{stem}_"))
        })
        .unwrap_or(false)
}

fn rust_workspace_heuristic_tests(
    root: &Path,
    file: &str,
    indexed_files: &[FileRecord],
) -> Vec<String> {
    let Some(changed) = indexed_files.iter().find(|record| record.path == file) else {
        return Vec::new();
    };
    if changed.language != Language::Rust || is_test_file(file) {
        return Vec::new();
    }
    let Some(crate_root) = rust_crate_root(file) else {
        return Vec::new();
    };
    indexed_files
        .iter()
        .filter(|record| record.language == Language::Rust)
        .filter(|record| record.path != file)
        .filter(|record| rust_crate_root(&record.path).as_deref() == Some(crate_root.as_str()))
        .filter(|record| {
            is_test_file(&record.path) || rust_file_contains_inline_tests(root, &record.path)
        })
        .map(|record| record.path.clone())
        .collect()
}

fn rust_crate_root(file: &str) -> Option<String> {
    let parts: Vec<&str> = file.split('/').collect();
    if parts.len() >= 2 && parts[0] == "crates" {
        return Some(format!("{}/{}", parts[0], parts[1]));
    }
    parts
        .iter()
        .position(|part| *part == "src")
        .map(|index| parts[..index].join("/"))
}

fn rust_file_contains_inline_tests(root: &Path, file: &str) -> bool {
    fs::read_to_string(root.join(file))
        .map(|text| text.contains("#[cfg(test)]") || text.contains("#[test]"))
        .unwrap_or(false)
}
