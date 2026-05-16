use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    TypeScript,
    JavaScript,
    Tsx,
    Jsx,
    Python,
    Go,
    Rust,
    Java,
    C,
    Cpp,
    CSharp,
    Php,
    Ruby,
    Swift,
    Kotlin,
    Dart,
    Svelte,
    Vue,
    Liquid,
    Pascal,
    Scala,
    MoonBit,
    Unknown,
}

impl Language {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TypeScript => "typescript",
            Self::JavaScript => "javascript",
            Self::Tsx => "tsx",
            Self::Jsx => "jsx",
            Self::Python => "python",
            Self::Go => "go",
            Self::Rust => "rust",
            Self::Java => "java",
            Self::C => "c",
            Self::Cpp => "cpp",
            Self::CSharp => "csharp",
            Self::Php => "php",
            Self::Ruby => "ruby",
            Self::Swift => "swift",
            Self::Kotlin => "kotlin",
            Self::Dart => "dart",
            Self::Svelte => "svelte",
            Self::Vue => "vue",
            Self::Liquid => "liquid",
            Self::Pascal => "pascal",
            Self::Scala => "scala",
            Self::MoonBit => "moonbit",
            Self::Unknown => "unknown",
        }
    }

    pub fn is_unknown(self) -> bool {
        self == Self::Unknown
    }
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for Language {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "typescript" => Self::TypeScript,
            "javascript" => Self::JavaScript,
            "tsx" => Self::Tsx,
            "jsx" => Self::Jsx,
            "python" => Self::Python,
            "go" => Self::Go,
            "rust" => Self::Rust,
            "java" => Self::Java,
            "c" => Self::C,
            "cpp" => Self::Cpp,
            "csharp" => Self::CSharp,
            "php" => Self::Php,
            "ruby" => Self::Ruby,
            "swift" => Self::Swift,
            "kotlin" => Self::Kotlin,
            "dart" => Self::Dart,
            "svelte" => Self::Svelte,
            "vue" => Self::Vue,
            "liquid" => Self::Liquid,
            "pascal" => Self::Pascal,
            "scala" => Self::Scala,
            "moonbit" => Self::MoonBit,
            _ => Self::Unknown,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    File,
    Module,
    Class,
    Struct,
    Interface,
    Trait,
    Protocol,
    Function,
    Method,
    Property,
    Field,
    Variable,
    Constant,
    Enum,
    EnumMember,
    TypeAlias,
    Namespace,
    Parameter,
    Import,
    Export,
    Route,
    Component,
}

impl NodeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Module => "module",
            Self::Class => "class",
            Self::Struct => "struct",
            Self::Interface => "interface",
            Self::Trait => "trait",
            Self::Protocol => "protocol",
            Self::Function => "function",
            Self::Method => "method",
            Self::Property => "property",
            Self::Field => "field",
            Self::Variable => "variable",
            Self::Constant => "constant",
            Self::Enum => "enum",
            Self::EnumMember => "enum_member",
            Self::TypeAlias => "type_alias",
            Self::Namespace => "namespace",
            Self::Parameter => "parameter",
            Self::Import => "import",
            Self::Export => "export",
            Self::Route => "route",
            Self::Component => "component",
        }
    }
}

impl fmt::Display for NodeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeKind {
    Contains,
    Calls,
    Imports,
    Exports,
    Extends,
    Implements,
    References,
    TypeOf,
    Returns,
    Instantiates,
    Overrides,
    Decorates,
}

impl EdgeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Contains => "contains",
            Self::Calls => "calls",
            Self::Imports => "imports",
            Self::Exports => "exports",
            Self::Extends => "extends",
            Self::Implements => "implements",
            Self::References => "references",
            Self::TypeOf => "type_of",
            Self::Returns => "returns",
            Self::Instantiates => "instantiates",
            Self::Overrides => "overrides",
            Self::Decorates => "decorates",
        }
    }
}

impl fmt::Display for EdgeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub kind: NodeKind,
    pub name: String,
    pub qualified_name: String,
    pub file_path: String,
    pub language: Language,
    pub start_line: i64,
    pub end_line: i64,
    pub start_column: i64,
    pub end_column: i64,
    pub docstring: Option<String>,
    pub signature: Option<String>,
    pub visibility: Option<String>,
    pub is_exported: bool,
    pub is_async: bool,
    pub is_static: bool,
    pub is_abstract: bool,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub id: Option<i64>,
    pub source: String,
    pub target: String,
    pub kind: EdgeKind,
    pub line: Option<i64>,
    pub col: Option<i64>,
    pub provenance: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnresolvedReference {
    pub from_node_id: String,
    pub reference_name: String,
    pub reference_kind: EdgeKind,
    pub line: i64,
    pub column: i64,
    pub file_path: String,
    pub language: Language,
}

#[derive(Debug, Clone)]
pub struct ExtractionResult {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub unresolved_references: Vec<UnresolvedReference>,
}

#[derive(Debug, Clone)]
pub struct FileRecord {
    pub path: String,
    pub content_hash: String,
    pub language: Language,
    pub size: u64,
    pub modified_at: i64,
    pub indexed_at: i64,
    pub node_count: i64,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct IndexResult {
    pub success: bool,
    pub files_indexed: i64,
    pub files_skipped: i64,
    pub files_errored: i64,
    pub nodes_created: i64,
    pub edges_created: i64,
    pub errors: Vec<String>,
    pub duration_ms: i64,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct GraphStats {
    pub file_count: i64,
    pub node_count: i64,
    pub edge_count: i64,
    pub db_size_bytes: i64,
    pub files_by_language: Vec<(String, i64)>,
    pub nodes_by_kind: Vec<(String, i64)>,
}

#[derive(Debug, Clone, Default)]
pub struct SearchOptions {
    pub limit: i64,
    pub kind: Option<NodeKind>,
    pub language: Option<Language>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    pub node: Node,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContextReport {
    pub query: String,
    pub search_terms: Vec<String>,
    pub matches: Vec<ContextMatch>,
    pub files: Vec<ContextFileSummary>,
    pub symbols: Vec<ContextSymbolSummary>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContextMatch {
    pub search_term: String,
    pub reason: String,
    pub score: f64,
    pub node: Node,
    pub code: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContextFileSummary {
    pub path: String,
    pub language: Language,
    pub match_count: i64,
    pub symbols: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContextSymbolSummary {
    pub name: String,
    pub kind: NodeKind,
    pub file_path: String,
    pub start_line: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AffectedReport {
    pub changed_files: Vec<String>,
    pub affected_tests: Vec<String>,
    pub debug: Vec<AffectedDebugEntry>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AffectedDebugEntry {
    pub changed_file: String,
    pub reason: String,
    pub matched_tests: Vec<String>,
    pub matched_by: AffectedMatchSources,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AffectedMatchSources {
    pub direct_test_input: Vec<String>,
    pub import_dependents: Vec<String>,
    pub moonbit_same_package: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NodeEdge {
    pub node: Node,
    pub edge: Edge,
}
