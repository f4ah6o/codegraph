use crate::types::*;
use anyhow::{Context, Result};
use regex::Regex;
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::str::FromStr;

pub struct Database {
    conn: Connection,
    path: PathBuf,
}

impl Database {
    pub fn initialize(path: impl AsRef<Path>) -> Result<Self> {
        let db = Self::open_raw(path)?;
        db.create_schema()?;
        Ok(db)
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let db = Self::open_raw(path)?;
        db.create_schema()?;
        Ok(db)
    }

    fn open_raw(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn =
            Connection::open(&path).with_context(|| format!("opening {}", path.display()))?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "busy_timeout", 120_000)?;
        Ok(Self { conn, path })
    }

    fn create_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS schema_versions (
                version INTEGER PRIMARY KEY,
                applied_at INTEGER NOT NULL,
                description TEXT
            );
            INSERT OR IGNORE INTO schema_versions (version, applied_at, description)
            VALUES (1, strftime('%s', 'now') * 1000, 'Rust schema');

            CREATE TABLE IF NOT EXISTS nodes (
                id TEXT PRIMARY KEY,
                kind TEXT NOT NULL,
                name TEXT NOT NULL,
                qualified_name TEXT NOT NULL,
                file_path TEXT NOT NULL,
                language TEXT NOT NULL,
                start_line INTEGER NOT NULL,
                end_line INTEGER NOT NULL,
                start_column INTEGER NOT NULL,
                end_column INTEGER NOT NULL,
                docstring TEXT,
                signature TEXT,
                visibility TEXT,
                is_exported INTEGER DEFAULT 0,
                is_async INTEGER DEFAULT 0,
                is_static INTEGER DEFAULT 0,
                is_abstract INTEGER DEFAULT 0,
                decorators TEXT,
                type_parameters TEXT,
                updated_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS edges (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                source TEXT NOT NULL,
                target TEXT NOT NULL,
                kind TEXT NOT NULL,
                metadata TEXT,
                line INTEGER,
                col INTEGER,
                provenance TEXT DEFAULT NULL,
                FOREIGN KEY (source) REFERENCES nodes(id) ON DELETE CASCADE,
                FOREIGN KEY (target) REFERENCES nodes(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS files (
                path TEXT PRIMARY KEY,
                content_hash TEXT NOT NULL,
                language TEXT NOT NULL,
                size INTEGER NOT NULL,
                modified_at INTEGER NOT NULL,
                indexed_at INTEGER NOT NULL,
                node_count INTEGER DEFAULT 0,
                errors TEXT
            );

            CREATE TABLE IF NOT EXISTS unresolved_refs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                from_node_id TEXT NOT NULL,
                reference_name TEXT NOT NULL,
                reference_kind TEXT NOT NULL,
                line INTEGER NOT NULL,
                col INTEGER NOT NULL,
                candidates TEXT,
                file_path TEXT NOT NULL DEFAULT '',
                language TEXT NOT NULL DEFAULT 'unknown',
                FOREIGN KEY (from_node_id) REFERENCES nodes(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_nodes_kind ON nodes(kind);
            CREATE INDEX IF NOT EXISTS idx_nodes_name ON nodes(name);
            CREATE INDEX IF NOT EXISTS idx_nodes_file_path ON nodes(file_path);
            CREATE INDEX IF NOT EXISTS idx_nodes_language ON nodes(language);
            CREATE INDEX IF NOT EXISTS idx_edges_kind ON edges(kind);
            CREATE INDEX IF NOT EXISTS idx_edges_source_kind ON edges(source, kind);
            CREATE INDEX IF NOT EXISTS idx_edges_target_kind ON edges(target, kind);
            CREATE INDEX IF NOT EXISTS idx_files_language ON files(language);
            CREATE INDEX IF NOT EXISTS idx_unresolved_name ON unresolved_refs(reference_name);
            "#,
        )?;
        Ok(())
    }

    pub fn clear_all(&self) -> Result<()> {
        self.conn.execute_batch(
            "DELETE FROM edges; DELETE FROM unresolved_refs; DELETE FROM nodes; DELETE FROM files;",
        )?;
        Ok(())
    }

    pub fn insert_file(&self, file: &FileRecord) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO files (path, content_hash, language, size, modified_at, indexed_at, node_count) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![file.path, file.content_hash, file.language.as_str(), file.size as i64, file.modified_at, file.indexed_at, file.node_count],
        )?;
        Ok(())
    }

    pub fn insert_nodes(&self, nodes: &[Node]) -> Result<()> {
        let mut stmt = self.conn.prepare(
            "INSERT OR REPLACE INTO nodes (id, kind, name, qualified_name, file_path, language, start_line, end_line, start_column, end_column, docstring, signature, visibility, is_exported, is_async, is_static, is_abstract, decorators, type_parameters, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, NULL, NULL, ?18)"
        )?;
        for n in nodes {
            stmt.execute(params![
                n.id,
                n.kind.as_str(),
                n.name,
                n.qualified_name,
                n.file_path,
                n.language.as_str(),
                n.start_line,
                n.end_line,
                n.start_column,
                n.end_column,
                n.docstring,
                n.signature,
                n.visibility,
                n.is_exported as i64,
                n.is_async as i64,
                n.is_static as i64,
                n.is_abstract as i64,
                n.updated_at
            ])?;
        }
        Ok(())
    }

    pub fn insert_edges(&self, edges: &[Edge]) -> Result<()> {
        let mut stmt = self.conn.prepare("INSERT INTO edges (source, target, kind, line, col, provenance) VALUES (?1, ?2, ?3, ?4, ?5, ?6)")?;
        for e in edges {
            stmt.execute(params![
                e.source,
                e.target,
                e.kind.as_str(),
                e.line,
                e.col,
                e.provenance
            ])?;
        }
        Ok(())
    }

    pub fn insert_unresolved_refs(&self, refs: &[UnresolvedReference]) -> Result<()> {
        let mut stmt = self.conn.prepare(
            "INSERT INTO unresolved_refs (from_node_id, reference_name, reference_kind, line, col, file_path, language) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"
        )?;
        for r in refs {
            stmt.execute(params![
                r.from_node_id,
                r.reference_name,
                r.reference_kind.as_str(),
                r.line,
                r.column,
                r.file_path,
                r.language.as_str()
            ])?;
        }
        Ok(())
    }

    pub fn resolve_references(&self, project_root: &Path) -> Result<()> {
        let indexed_files = self.indexed_file_set()?;
        let aliases = load_project_aliases(project_root).unwrap_or_default();
        let mut refs = self.conn.prepare("SELECT from_node_id, reference_name, reference_kind, line, col, file_path, language FROM unresolved_refs")?;
        let rows = refs.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<i64>>(3)?,
                row.get::<_, Option<i64>>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
            ))
        })?;
        for row in rows {
            let (from, name, kind, line, col, file_path, lang) = row?;
            let language = Language::from_str(&lang).unwrap_or(Language::Unknown);
            let mut target =
                self.resolve_reference_path(&name, &file_path, language, &indexed_files, &aliases)?;
            if target.is_none() {
                target = self.resolve_reference_by_name(&from, &name, &lang)?;
            }
            if let Some(target) = target {
                self.conn.execute(
                    "INSERT INTO edges (source, target, kind, line, col, provenance) VALUES (?1, ?2, ?3, ?4, ?5, 'resolver')",
                    params![from, target, kind, line, col],
                )?;
            }
        }
        Ok(())
    }

    pub fn resolve_references_by_name(&self) -> Result<()> {
        self.resolve_references(Path::new("."))
    }

    fn indexed_file_set(&self) -> Result<BTreeSet<String>> {
        let mut stmt = self.conn.prepare("SELECT path FROM files")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        let mut out = BTreeSet::new();
        for row in rows {
            out.insert(normalize_path(&row?));
        }
        Ok(out)
    }

    fn resolve_reference_path(
        &self,
        reference_name: &str,
        from_file: &str,
        language: Language,
        indexed_files: &BTreeSet<String>,
        aliases: &[PathAlias],
    ) -> Result<Option<String>> {
        let Some(path) =
            resolve_import_path(reference_name, from_file, language, indexed_files, aliases)
        else {
            return Ok(None);
        };
        self.conn
            .query_row(
                "SELECT id FROM nodes WHERE kind = 'file' AND file_path = ?1 LIMIT 1",
                [path],
                |row| row.get(0),
            )
            .optional()
            .map_err(Into::into)
    }

    fn resolve_reference_by_name(
        &self,
        from_node_id: &str,
        name: &str,
        language: &str,
    ) -> Result<Option<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, kind, file_path FROM nodes WHERE name = ?1 AND language = ?2 AND id != ?3",
        )?;
        let rows = stmt.query_map(params![name, language, from_node_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;
        let mut candidates = Vec::new();
        for row in rows {
            candidates.push(row?);
        }
        if candidates.is_empty() {
            return Ok(None);
        }
        candidates.sort_by(|a, b| {
            node_resolution_rank(&a.1)
                .cmp(&node_resolution_rank(&b.1))
                .then_with(|| a.2.cmp(&b.2))
                .then_with(|| a.0.cmp(&b.0))
        });
        let best_rank = node_resolution_rank(&candidates[0].1);
        let best_count = candidates
            .iter()
            .filter(|(_, kind, _)| node_resolution_rank(kind) == best_rank)
            .count();
        if best_count == 1 {
            Ok(Some(candidates[0].0.clone()))
        } else {
            Ok(None)
        }
    }

    pub fn edge_count(&self) -> Result<i64> {
        Ok(self
            .conn
            .query_row("SELECT COUNT(*) FROM edges", [], |r| r.get(0))?)
    }

    pub fn stats(&self) -> Result<GraphStats> {
        let file_count = self
            .conn
            .query_row("SELECT COUNT(*) FROM files", [], |r| r.get(0))?;
        let node_count = self
            .conn
            .query_row("SELECT COUNT(*) FROM nodes", [], |r| r.get(0))?;
        let edge_count = self
            .conn
            .query_row("SELECT COUNT(*) FROM edges", [], |r| r.get(0))?;
        let db_size_bytes = std::fs::metadata(&self.path)
            .map(|m| m.len() as i64)
            .unwrap_or_default();
        let oldest_indexed_at =
            self.conn
                .query_row("SELECT MIN(indexed_at) FROM files", [], |r| r.get(0))?;
        let last_indexed_at =
            self.conn
                .query_row("SELECT MAX(indexed_at) FROM files", [], |r| r.get(0))?;
        let newest_modified_at =
            self.conn
                .query_row("SELECT MAX(modified_at) FROM files", [], |r| r.get(0))?;
        let stale_file_count = self.conn.query_row(
            "SELECT COUNT(*) FROM files WHERE modified_at > indexed_at",
            [],
            |r| r.get(0),
        )?;
        let files_by_language = grouped_counts(
            &self.conn,
            "SELECT language, COUNT(*) FROM files GROUP BY language",
        )?;
        let nodes_by_kind =
            grouped_counts(&self.conn, "SELECT kind, COUNT(*) FROM nodes GROUP BY kind")?;
        Ok(GraphStats {
            file_count,
            node_count,
            edge_count,
            db_size_bytes,
            oldest_indexed_at,
            last_indexed_at,
            newest_modified_at,
            stale_file_count,
            files_by_language,
            nodes_by_kind,
        })
    }

    pub fn search_nodes(&self, query: &str, options: SearchOptions) -> Result<Vec<SearchResult>> {
        let limit = if options.limit <= 0 {
            10
        } else {
            options.limit
        };
        let fetch_limit = (limit * 5).max(limit).min(500);
        let pattern = format!("%{}%", query);
        let exact = query.to_string();
        let prefix = format!("{}%", query);

        let base = "SELECT id, kind, name, qualified_name, file_path, language, start_line, end_line, start_column, end_column, docstring, signature, visibility, is_exported, is_async, is_static, is_abstract, updated_at FROM nodes";
        let order = " ORDER BY CASE WHEN name = ? THEN 0 WHEN name LIKE ? THEN 1 ELSE 2 END, length(name) LIMIT ?";

        let rows = match (options.kind, options.language) {
            (Some(k), Some(l)) => {
                let sql = format!("{base} WHERE (name LIKE ? OR qualified_name LIKE ? OR signature LIKE ? OR file_path LIKE ?) AND kind = ? AND language = ?{order}");
                let mut stmt = self.conn.prepare(&sql)?;
                let nodes = collect_nodes(stmt.query_map(
                    params![
                        pattern,
                        pattern,
                        pattern,
                        pattern,
                        k.as_str(),
                        l.as_str(),
                        exact,
                        prefix,
                        fetch_limit
                    ],
                    node_from_row,
                )?)?;
                nodes
            }
            (Some(k), None) => {
                let sql = format!("{base} WHERE (name LIKE ? OR qualified_name LIKE ? OR signature LIKE ? OR file_path LIKE ?) AND kind = ?{order}");
                let mut stmt = self.conn.prepare(&sql)?;
                let nodes = collect_nodes(stmt.query_map(
                    params![
                        pattern,
                        pattern,
                        pattern,
                        pattern,
                        k.as_str(),
                        exact,
                        prefix,
                        fetch_limit
                    ],
                    node_from_row,
                )?)?;
                nodes
            }
            (None, Some(l)) => {
                let sql = format!("{base} WHERE (name LIKE ? OR qualified_name LIKE ? OR signature LIKE ? OR file_path LIKE ?) AND language = ?{order}");
                let mut stmt = self.conn.prepare(&sql)?;
                let nodes = collect_nodes(stmt.query_map(
                    params![
                        pattern,
                        pattern,
                        pattern,
                        pattern,
                        l.as_str(),
                        exact,
                        prefix,
                        fetch_limit
                    ],
                    node_from_row,
                )?)?;
                nodes
            }
            (None, None) => {
                let sql = format!("{base} WHERE (name LIKE ? OR qualified_name LIKE ? OR signature LIKE ? OR file_path LIKE ?){order}");
                let mut stmt = self.conn.prepare(&sql)?;
                let nodes = collect_nodes(stmt.query_map(
                    params![
                        pattern,
                        pattern,
                        pattern,
                        pattern,
                        exact,
                        prefix,
                        fetch_limit
                    ],
                    node_from_row,
                )?)?;
                nodes
            }
        };
        let mut results = rows
            .into_iter()
            .map(|node| SearchResult {
                score: search_score(query, &node),
                node,
            })
            .collect::<Vec<_>>();
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.node.name.len().cmp(&b.node.name.len()))
                .then_with(|| a.node.file_path.cmp(&b.node.file_path))
                .then_with(|| a.node.start_line.cmp(&b.node.start_line))
        });
        results.truncate(limit as usize);
        Ok(results)
    }

    pub fn get_node(&self, id: &str) -> Result<Option<Node>> {
        self.conn
            .query_row("SELECT id, kind, name, qualified_name, file_path, language, start_line, end_line, start_column, end_column, docstring, signature, visibility, is_exported, is_async, is_static, is_abstract, updated_at FROM nodes WHERE id = ?1", [id], node_from_row)
            .optional()
            .map_err(Into::into)
    }

    pub fn get_nodes_by_name(&self, name: &str, limit: i64) -> Result<Vec<Node>> {
        let mut stmt = self.conn.prepare("SELECT id, kind, name, qualified_name, file_path, language, start_line, end_line, start_column, end_column, docstring, signature, visibility, is_exported, is_async, is_static, is_abstract, updated_at FROM nodes WHERE name = ?1 ORDER BY file_path, start_line LIMIT ?2")?;
        let nodes = collect_nodes(stmt.query_map(params![name, limit], node_from_row)?)?;
        Ok(nodes)
    }

    pub fn get_all_files(&self) -> Result<Vec<FileRecord>> {
        let mut stmt = self.conn.prepare("SELECT path, content_hash, language, size, modified_at, indexed_at, node_count FROM files ORDER BY path")?;
        let rows = stmt.query_map([], |row| {
            let language: String = row.get(2)?;
            Ok(FileRecord {
                path: row.get(0)?,
                content_hash: row.get(1)?,
                language: Language::from_str(&language).unwrap_or(Language::Unknown),
                size: row.get::<_, i64>(3)? as u64,
                modified_at: row.get(4)?,
                indexed_at: row.get(5)?,
                node_count: row.get(6)?,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn get_nodes_in_file(&self, file_path: &str) -> Result<Vec<Node>> {
        let mut stmt = self.conn.prepare("SELECT id, kind, name, qualified_name, file_path, language, start_line, end_line, start_column, end_column, docstring, signature, visibility, is_exported, is_async, is_static, is_abstract, updated_at FROM nodes WHERE file_path = ?1 ORDER BY start_line, start_column")?;
        let nodes = collect_nodes(stmt.query_map([file_path], node_from_row)?)?;
        Ok(nodes)
    }

    pub fn get_incoming_edges(
        &self,
        node_id: &str,
        kinds: Option<&[EdgeKind]>,
    ) -> Result<Vec<Edge>> {
        self.get_edges(node_id, EdgeDirection::Incoming, kinds)
    }

    pub fn get_outgoing_edges(
        &self,
        node_id: &str,
        kinds: Option<&[EdgeKind]>,
    ) -> Result<Vec<Edge>> {
        self.get_edges(node_id, EdgeDirection::Outgoing, kinds)
    }

    pub fn get_file_dependents(&self, file_path: &str) -> Result<Vec<String>> {
        let mut out = std::collections::BTreeSet::new();
        for node in self.get_nodes_in_file(file_path)? {
            let edges = self.get_incoming_edges(
                &node.id,
                Some(&[
                    EdgeKind::Calls,
                    EdgeKind::References,
                    EdgeKind::Imports,
                    EdgeKind::Extends,
                    EdgeKind::Implements,
                ]),
            )?;
            for edge in edges {
                if let Some(source) = self.get_node(&edge.source)? {
                    if source.file_path != file_path {
                        out.insert(source.file_path);
                    }
                }
            }
        }
        Ok(out.into_iter().collect())
    }

    fn get_edges(
        &self,
        node_id: &str,
        direction: EdgeDirection,
        kinds: Option<&[EdgeKind]>,
    ) -> Result<Vec<Edge>> {
        let column = match direction {
            EdgeDirection::Incoming => "target",
            EdgeDirection::Outgoing => "source",
        };
        let mut sql = format!(
            "SELECT id, source, target, kind, line, col, provenance FROM edges WHERE {column} = ?"
        );
        if let Some(kinds) = kinds {
            if !kinds.is_empty() {
                sql.push_str(" AND kind IN (");
                sql.push_str(
                    &std::iter::repeat("?")
                        .take(kinds.len())
                        .collect::<Vec<_>>()
                        .join(","),
                );
                sql.push(')');
            }
        }
        sql.push_str(" ORDER BY id");

        let mut values = vec![node_id.to_string()];
        if let Some(kinds) = kinds {
            values.extend(kinds.iter().map(|k| k.as_str().to_string()));
        }
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(values.iter()), edge_from_row)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }
}

enum EdgeDirection {
    Incoming,
    Outgoing,
}

fn collect_nodes(
    rows: rusqlite::MappedRows<'_, impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<Node>>,
) -> Result<Vec<Node>> {
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

fn grouped_counts(conn: &Connection, sql: &str) -> Result<Vec<(String, i64)>> {
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

fn node_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Node> {
    let kind: String = row.get(1)?;
    let language: String = row.get(5)?;
    Ok(Node {
        id: row.get(0)?,
        kind: parse_kind(&kind),
        name: row.get(2)?,
        qualified_name: row.get(3)?,
        file_path: row.get(4)?,
        language: Language::from_str(&language).unwrap_or(Language::Unknown),
        start_line: row.get(6)?,
        end_line: row.get(7)?,
        start_column: row.get(8)?,
        end_column: row.get(9)?,
        docstring: row.get(10)?,
        signature: row.get(11)?,
        visibility: row.get(12)?,
        is_exported: row.get::<_, i64>(13)? != 0,
        is_async: row.get::<_, i64>(14)? != 0,
        is_static: row.get::<_, i64>(15)? != 0,
        is_abstract: row.get::<_, i64>(16)? != 0,
        updated_at: row.get(17)?,
    })
}

fn edge_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Edge> {
    let kind: String = row.get(3)?;
    Ok(Edge {
        id: row.get(0)?,
        source: row.get(1)?,
        target: row.get(2)?,
        kind: parse_edge_kind(&kind),
        line: row.get(4)?,
        col: row.get(5)?,
        provenance: row.get(6)?,
    })
}

fn search_score(query: &str, node: &Node) -> f64 {
    let query = query.to_ascii_lowercase();
    let name = node.name.to_ascii_lowercase();
    let qualified = node.qualified_name.to_ascii_lowercase();
    let file_path = node.file_path.to_ascii_lowercase();
    let signature = node
        .signature
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase();

    if name == query {
        100.0
    } else if name.starts_with(&query) {
        90.0
    } else if qualified.contains(&query) {
        80.0
    } else if file_path.contains(&query) {
        70.0
    } else if signature.contains(&query) {
        60.0
    } else {
        10.0
    }
}

#[derive(Debug, Clone)]
struct PathAlias {
    prefix: String,
    suffix: String,
    replacements: Vec<String>,
    has_wildcard: bool,
}

fn resolve_import_path(
    reference_name: &str,
    from_file: &str,
    language: Language,
    indexed_files: &BTreeSet<String>,
    aliases: &[PathAlias],
) -> Option<String> {
    if is_external_import(reference_name, language, aliases) {
        return None;
    }

    let mut bases = Vec::new();
    if reference_name.starts_with('.') {
        bases.push(join_normalized(parent_dir(from_file), reference_name));
    } else {
        bases.extend(apply_path_aliases(reference_name, aliases));
        for (prefix, replacement) in [
            ("@/", "src/"),
            ("~/", "src/"),
            ("@src/", "src/"),
            ("src/", "src/"),
            ("@app/", "app/"),
            ("app/", "app/"),
        ] {
            if reference_name.starts_with(prefix) {
                bases.push(format!(
                    "{}{}",
                    replacement,
                    reference_name.trim_start_matches(prefix)
                ));
            }
        }
        bases.push(reference_name.to_string());
    }

    for base in bases {
        for candidate in import_candidates(&base, language) {
            let candidate = normalize_path(&candidate);
            if indexed_files.contains(&candidate) {
                return Some(candidate);
            }
        }
    }
    None
}

fn is_external_import(reference_name: &str, language: Language, aliases: &[PathAlias]) -> bool {
    if reference_name.starts_with('.') || reference_name.contains('/') {
        return false;
    }
    if aliases
        .iter()
        .any(|alias| alias_matches(reference_name, alias))
    {
        return false;
    }
    match language {
        Language::TypeScript | Language::JavaScript | Language::Tsx | Language::Jsx => true,
        Language::Python => matches!(
            reference_name.split('.').next().unwrap_or(reference_name),
            "os" | "sys" | "json" | "re" | "math" | "datetime" | "collections" | "typing"
        ),
        _ => false,
    }
}

fn import_candidates(base: &str, language: Language) -> Vec<String> {
    let mut out = Vec::new();
    out.push(base.to_string());
    let exts: &[&str] = match language {
        Language::TypeScript => &[
            ".ts",
            ".tsx",
            ".d.ts",
            ".js",
            ".jsx",
            "/index.ts",
            "/index.tsx",
            "/index.js",
        ],
        Language::JavaScript => &[".js", ".jsx", ".mjs", ".cjs", "/index.js", "/index.jsx"],
        Language::Tsx => &[
            ".tsx",
            ".ts",
            ".d.ts",
            ".js",
            ".jsx",
            "/index.tsx",
            "/index.ts",
            "/index.js",
        ],
        Language::Jsx => &[".jsx", ".js", "/index.jsx", "/index.js"],
        Language::Vue => &[".vue", ".ts", ".js", "/index.vue", "/index.ts", "/index.js"],
        Language::Svelte => &[
            ".svelte",
            ".ts",
            ".js",
            "/index.svelte",
            "/index.ts",
            "/index.js",
        ],
        Language::Liquid => &[".liquid"],
        Language::Python => &[".py", "/__init__.py"],
        Language::Rust => &[".rs", "/mod.rs"],
        Language::Go => &[".go"],
        Language::Java => &[".java"],
        Language::Kotlin => &[".kt", ".kts"],
        Language::CSharp => &[".cs"],
        Language::Php => &[".php"],
        Language::Ruby => &[".rb"],
        Language::Dart => &[".dart"],
        Language::Pascal => &[".pas", ".pp"],
        Language::Scala => &[".scala"],
        _ => &[],
    };
    if !Path::new(base)
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| !ext.is_empty())
    {
        out.extend(exts.iter().map(|ext| format!("{base}{ext}")));
    }
    out
}

fn load_project_aliases(project_root: &Path) -> Result<Vec<PathAlias>> {
    for name in ["tsconfig.json", "jsconfig.json"] {
        let path = project_root.join(name);
        if !path.exists() {
            continue;
        }
        let content = std::fs::read_to_string(path)?;
        let value: Value = serde_json::from_str(&strip_jsonc(&content))?;
        let Some(paths) = value
            .pointer("/compilerOptions/paths")
            .and_then(|paths| paths.as_object())
        else {
            return Ok(Vec::new());
        };
        let mut aliases = Vec::new();
        for (pattern, replacements) in paths {
            let Some(items) = replacements.as_array() else {
                continue;
            };
            let replacements = items
                .iter()
                .filter_map(|item| item.as_str().map(normalize_path))
                .collect::<Vec<_>>();
            if replacements.is_empty() {
                continue;
            }
            let (prefix, suffix, has_wildcard) = split_alias_pattern(pattern);
            aliases.push(PathAlias {
                prefix,
                suffix,
                replacements,
                has_wildcard,
            });
        }
        aliases.sort_by(|a, b| b.prefix.len().cmp(&a.prefix.len()));
        return Ok(aliases);
    }
    Ok(Vec::new())
}

fn split_alias_pattern(pattern: &str) -> (String, String, bool) {
    if let Some(index) = pattern.find('*') {
        (
            pattern[..index].to_string(),
            pattern[index + 1..].to_string(),
            true,
        )
    } else {
        (pattern.to_string(), String::new(), false)
    }
}

fn apply_path_aliases(reference_name: &str, aliases: &[PathAlias]) -> Vec<String> {
    let mut out = Vec::new();
    for alias in aliases {
        if !alias_matches(reference_name, alias) {
            continue;
        }
        let captured = if alias.has_wildcard {
            &reference_name[alias.prefix.len()..reference_name.len() - alias.suffix.len()]
        } else {
            ""
        };
        for replacement in &alias.replacements {
            out.push(if alias.has_wildcard {
                replacement.replace('*', captured)
            } else {
                replacement.clone()
            });
        }
        break;
    }
    out
}

fn alias_matches(reference_name: &str, alias: &PathAlias) -> bool {
    if !reference_name.starts_with(&alias.prefix) || !reference_name.ends_with(&alias.suffix) {
        return false;
    }
    alias.has_wildcard || reference_name == alias.prefix
}

fn strip_jsonc(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    let mut in_string = false;
    while let Some(ch) = chars.next() {
        if in_string {
            out.push(ch);
            if ch == '\\' {
                if let Some(next) = chars.next() {
                    out.push(next);
                }
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        if ch == '"' {
            in_string = true;
            out.push(ch);
            continue;
        }
        if ch == '/' && chars.peek() == Some(&'/') {
            for next in chars.by_ref() {
                if next == '\n' {
                    out.push('\n');
                    break;
                }
            }
            continue;
        }
        if ch == '/' && chars.peek() == Some(&'*') {
            chars.next();
            while let Some(next) = chars.next() {
                if next == '*' && chars.peek() == Some(&'/') {
                    chars.next();
                    break;
                }
            }
            continue;
        }
        out.push(ch);
    }
    Regex::new(r",(\s*[}\]])")
        .unwrap()
        .replace_all(&out, "$1")
        .to_string()
}

fn parent_dir(path: &str) -> &str {
    path.rsplit_once('/')
        .map(|(parent, _)| parent)
        .unwrap_or("")
}

fn join_normalized(parent: &str, child: &str) -> String {
    let mut parts = Vec::new();
    let joined = format!("{parent}/{child}");
    for part in joined.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            _ => parts.push(part.to_string()),
        }
    }
    parts.join("/")
}

fn normalize_path(path: &str) -> String {
    join_normalized("", &path.replace('\\', "/"))
}

fn node_resolution_rank(kind: &str) -> i64 {
    match kind {
        "function" => 0,
        "method" => 1,
        "component" => 2,
        "class" => 3,
        "struct" => 4,
        "interface" => 5,
        "trait" => 6,
        "module" => 7,
        "file" => 8,
        _ => 20,
    }
}

fn parse_kind(s: &str) -> NodeKind {
    match s {
        "file" => NodeKind::File,
        "module" => NodeKind::Module,
        "class" => NodeKind::Class,
        "struct" => NodeKind::Struct,
        "interface" => NodeKind::Interface,
        "trait" => NodeKind::Trait,
        "protocol" => NodeKind::Protocol,
        "function" => NodeKind::Function,
        "method" => NodeKind::Method,
        "property" => NodeKind::Property,
        "field" => NodeKind::Field,
        "variable" => NodeKind::Variable,
        "constant" => NodeKind::Constant,
        "enum" => NodeKind::Enum,
        "enum_member" => NodeKind::EnumMember,
        "type_alias" => NodeKind::TypeAlias,
        "namespace" => NodeKind::Namespace,
        "parameter" => NodeKind::Parameter,
        "import" => NodeKind::Import,
        "export" => NodeKind::Export,
        "route" => NodeKind::Route,
        "component" => NodeKind::Component,
        _ => NodeKind::Variable,
    }
}

fn parse_edge_kind(s: &str) -> EdgeKind {
    match s {
        "contains" => EdgeKind::Contains,
        "calls" => EdgeKind::Calls,
        "imports" => EdgeKind::Imports,
        "exports" => EdgeKind::Exports,
        "extends" => EdgeKind::Extends,
        "implements" => EdgeKind::Implements,
        "references" => EdgeKind::References,
        "type_of" => EdgeKind::TypeOf,
        "returns" => EdgeKind::Returns,
        "instantiates" => EdgeKind::Instantiates,
        "overrides" => EdgeKind::Overrides,
        "decorates" => EdgeKind::Decorates,
        _ => EdgeKind::References,
    }
}
