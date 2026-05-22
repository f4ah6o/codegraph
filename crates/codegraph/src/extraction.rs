use crate::config::CodeGraphConfig;
use crate::types::*;
use regex::Regex;
use std::path::Path;
use tree_sitter::{Node as SyntaxNode, Parser};

type ExtractorFn = fn(
    file_path: &str,
    source: &str,
    language: Language,
    now: i64,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
    refs: &mut Vec<UnresolvedReference>,
);

#[derive(Clone, Copy)]
struct LanguageExtractor {
    name: &'static str,
    languages: &'static [Language],
    extract: ExtractorFn,
}

const RUST_LANGUAGES: &[Language] = &[Language::Rust];
const MOONBIT_LANGUAGES: &[Language] = &[Language::MoonBit];
const PYTHON_LANGUAGES: &[Language] = &[Language::Python];
const GO_LANGUAGES: &[Language] = &[Language::Go];
const JAVA_KOTLIN_LANGUAGES: &[Language] = &[Language::Java, Language::Kotlin];
const CSHARP_LANGUAGES: &[Language] = &[Language::CSharp];
const TYPESCRIPT_JAVASCRIPT_LANGUAGES: &[Language] = &[
    Language::TypeScript,
    Language::Tsx,
    Language::JavaScript,
    Language::Jsx,
];
const GENERIC_LANGUAGES: &[Language] = &[
    Language::C,
    Language::Cpp,
    Language::Php,
    Language::Ruby,
    Language::Swift,
    Language::Dart,
    Language::Svelte,
    Language::Vue,
    Language::Liquid,
    Language::Pascal,
    Language::Scala,
    Language::Unknown,
];

const LANGUAGE_EXTRACTORS: &[LanguageExtractor] = &[
    LanguageExtractor {
        name: "rust",
        languages: RUST_LANGUAGES,
        extract: extract_rust_entry,
    },
    LanguageExtractor {
        name: "moonbit",
        languages: MOONBIT_LANGUAGES,
        extract: extract_moonbit_entry,
    },
    LanguageExtractor {
        name: "typescript_javascript",
        languages: TYPESCRIPT_JAVASCRIPT_LANGUAGES,
        extract: extract_typescript_javascript_entry,
    },
    LanguageExtractor {
        name: "python",
        languages: PYTHON_LANGUAGES,
        extract: extract_python_entry,
    },
    LanguageExtractor {
        name: "go",
        languages: GO_LANGUAGES,
        extract: extract_go_entry,
    },
    LanguageExtractor {
        name: "java_kotlin",
        languages: JAVA_KOTLIN_LANGUAGES,
        extract: extract_java_kotlin_entry,
    },
    LanguageExtractor {
        name: "csharp",
        languages: CSHARP_LANGUAGES,
        extract: extract_csharp_entry,
    },
    LanguageExtractor {
        name: "generic",
        languages: GENERIC_LANGUAGES,
        extract: extract_generic_entry,
    },
];

pub fn should_include_file(path: &Path, config: &CodeGraphConfig) -> bool {
    let s = path.to_string_lossy().replace('\\', "/");
    if s.starts_with(".codegraph/") {
        return false;
    }
    if config.exclude.iter().any(|p| glob_match(p, &s)) {
        return false;
    }
    config.include.iter().any(|p| glob_match(p, &s))
}

fn glob_match(pattern: &str, path: &str) -> bool {
    let suffix = pattern.strip_prefix("**/*.");
    if let Some(ext) = suffix {
        return path.ends_with(&format!(".{}", ext));
    }
    if let Some(dir) = pattern
        .strip_prefix("**/")
        .and_then(|p| p.strip_suffix("/**"))
    {
        return path.contains(&format!("{}/", dir)) || path == dir;
    }
    if let Some(suffix) = pattern.strip_prefix("**/") {
        return path.ends_with(suffix);
    }
    pattern == path
}

pub fn detect_language(path: &Path, _source: &str) -> Language {
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_lowercase();
    if name == "moon.mod.json" || name == "moon.pkg.json" || name == "moon.pkg" {
        return Language::MoonBit;
    }
    if name.ends_with(".mbt.md") {
        return Language::MoonBit;
    }
    match path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_lowercase()
        .as_str()
    {
        "ts" => Language::TypeScript,
        "tsx" => Language::Tsx,
        "js" | "mjs" | "cjs" => Language::JavaScript,
        "jsx" => Language::Jsx,
        "py" | "pyw" => Language::Python,
        "go" => Language::Go,
        "rs" => Language::Rust,
        "java" => Language::Java,
        "c" | "h" => Language::C,
        "cpp" | "cc" | "cxx" | "hpp" | "hxx" => Language::Cpp,
        "cs" => Language::CSharp,
        "php" => Language::Php,
        "rb" | "rake" => Language::Ruby,
        "swift" => Language::Swift,
        "kt" | "kts" => Language::Kotlin,
        "dart" => Language::Dart,
        "svelte" => Language::Svelte,
        "vue" => Language::Vue,
        "liquid" => Language::Liquid,
        "pas" | "dpr" | "dpk" | "lpr" | "dfm" | "fmx" => Language::Pascal,
        "scala" | "sc" => Language::Scala,
        "mbt" | "mbti" => Language::MoonBit,
        _ => Language::Unknown,
    }
}

pub fn extract_from_source(path: &Path, source: &str, language: Language) -> ExtractionResult {
    let file_path = path.to_string_lossy().replace('\\', "/");
    let now = now_ms();
    let mut nodes = vec![Node {
        id: format!("file:{}", file_path),
        kind: NodeKind::File,
        name: path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(&file_path)
            .to_string(),
        qualified_name: file_path.clone(),
        file_path: file_path.clone(),
        language,
        start_line: 1,
        end_line: source.lines().count().max(1) as i64,
        start_column: 0,
        end_column: 0,
        docstring: None,
        signature: None,
        visibility: None,
        is_exported: false,
        is_async: false,
        is_static: false,
        is_abstract: false,
        updated_at: now,
    }];
    let mut edges = Vec::new();
    let mut refs = Vec::new();

    let extractor = extractor_for_language(language);
    (extractor.extract)(
        &file_path, &source, language, now, &mut nodes, &mut edges, &mut refs,
    );

    ExtractionResult {
        nodes,
        edges,
        unresolved_references: refs,
    }
}

pub fn registered_extractor_name(language: Language) -> &'static str {
    extractor_for_language(language).name
}

fn extractor_for_language(language: Language) -> LanguageExtractor {
    LANGUAGE_EXTRACTORS
        .iter()
        .copied()
        .find(|extractor| extractor.languages.contains(&language))
        .unwrap_or(LanguageExtractor {
            name: "generic",
            languages: &[],
            extract: extract_generic_entry,
        })
}

fn extract_rust_entry(
    file_path: &str,
    source: &str,
    _language: Language,
    now: i64,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
    refs: &mut Vec<UnresolvedReference>,
) {
    extract_rust(file_path, source, now, nodes, edges, refs);
}

fn extract_moonbit_entry(
    file_path: &str,
    source: &str,
    _language: Language,
    now: i64,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
    refs: &mut Vec<UnresolvedReference>,
) {
    extract_moonbit(file_path, source, now, nodes, edges, refs);
}

fn extract_typescript_javascript_entry(
    file_path: &str,
    source: &str,
    language: Language,
    now: i64,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
    refs: &mut Vec<UnresolvedReference>,
) {
    extract_typescript_javascript(file_path, source, language, now, nodes, edges, refs);
}

fn extract_python_entry(
    file_path: &str,
    source: &str,
    _language: Language,
    now: i64,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
    refs: &mut Vec<UnresolvedReference>,
) {
    extract_python(file_path, source, now, nodes, edges, refs);
}

fn extract_go_entry(
    file_path: &str,
    source: &str,
    _language: Language,
    now: i64,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
    refs: &mut Vec<UnresolvedReference>,
) {
    extract_go(file_path, source, now, nodes, edges, refs);
}

fn extract_java_kotlin_entry(
    file_path: &str,
    source: &str,
    language: Language,
    now: i64,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
    refs: &mut Vec<UnresolvedReference>,
) {
    extract_java_kotlin(file_path, source, language, now, nodes, edges, refs);
}

fn extract_csharp_entry(
    file_path: &str,
    source: &str,
    _language: Language,
    now: i64,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
    refs: &mut Vec<UnresolvedReference>,
) {
    extract_csharp(file_path, source, now, nodes, edges, refs);
}

fn extract_generic_entry(
    file_path: &str,
    source: &str,
    language: Language,
    now: i64,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
    refs: &mut Vec<UnresolvedReference>,
) {
    extract_generic(file_path, source, language, now, nodes, edges, refs);
}

fn extract_typescript_javascript(
    file_path: &str,
    source: &str,
    language: Language,
    now: i64,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
    refs: &mut Vec<UnresolvedReference>,
) {
    add_regex_nodes(
        file_path,
        source,
        language,
        now,
        nodes,
        edges,
        r"(?m)^\s*(export\s+)?(?:async\s+)?function\s+([A-Za-z_$][A-Za-z0-9_$]*)\s*([^{;]*)",
        NodeKind::Function,
    );
    add_regex_nodes(
        file_path,
        source,
        language,
        now,
        nodes,
        edges,
        r"(?m)^\s*(export\s+)?class\s+([A-Za-z_$][A-Za-z0-9_$]*)",
        NodeKind::Class,
    );
    add_regex_nodes(
        file_path,
        source,
        language,
        now,
        nodes,
        edges,
        r"(?m)^\s*(export\s+)?interface\s+([A-Za-z_$][A-Za-z0-9_$]*)",
        NodeKind::Interface,
    );
    add_regex_nodes(
        file_path,
        source,
        language,
        now,
        nodes,
        edges,
        r"(?m)^\s*(export\s+)?type\s+([A-Za-z_$][A-Za-z0-9_$]*)\s*=",
        NodeKind::TypeAlias,
    );
    add_ts_js_arrow_functions(file_path, source, language, now, nodes, edges);
    add_ts_js_imports(file_path, source, language, now, nodes, edges, refs);
    add_tsx_jsx_components(file_path, language, now, nodes, edges);
    add_call_refs(
        file_path,
        source,
        language,
        nodes,
        refs,
        r"([A-Za-z_$][A-Za-z0-9_$.]*)\s*\(",
    );
}

fn add_ts_js_arrow_functions(
    file_path: &str,
    source: &str,
    language: Language,
    now: i64,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
) {
    let re = Regex::new(
        r"(?m)^\s*(export\s+)?const\s+([A-Za-z_$][A-Za-z0-9_$]*)\s*(?::[^=]+)?=\s*(?:async\s+)?(?:\([^)]*\)|[A-Za-z_$][A-Za-z0-9_$]*)(?:\s*:\s*[^=;\n]+)?\s*=>",
    )
    .unwrap();
    for cap in re.captures_iter(source) {
        let name_match = cap.get(2).unwrap();
        let mut node = make_node(
            file_path,
            language,
            NodeKind::Function,
            name_match.as_str(),
            line_for(source, name_match.start()),
            0,
            now,
            cap.get(0).map(|m| m.as_str().trim().to_string()),
        );
        node.is_exported = cap.get(1).is_some();
        node.visibility = node.is_exported.then(|| "public".to_string());
        node.is_async = cap
            .get(0)
            .map(|m| m.as_str().contains("async"))
            .unwrap_or(false);
        add_contains(nodes, edges, &node);
        nodes.push(node);
    }
}

fn add_ts_js_imports(
    file_path: &str,
    source: &str,
    language: Language,
    now: i64,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
    refs: &mut Vec<UnresolvedReference>,
) {
    let re =
        Regex::new(r#"(?m)^\s*import(?:\s+type)?(?:\s+[^;\n]*?\s+from)?\s+['"]([^'"]+)['"]\s*;?"#)
            .unwrap();
    for cap in re.captures_iter(source) {
        let module = cap.get(1).unwrap();
        let signature = cap.get(0).unwrap().as_str().trim().to_string();
        let node = make_node(
            file_path,
            language,
            NodeKind::Import,
            module.as_str(),
            line_for(source, module.start()),
            0,
            now,
            Some(signature),
        );
        add_contains(nodes, edges, &node);
        refs.push(unresolved(
            &nodes[0].id,
            module.as_str(),
            EdgeKind::Imports,
            file_path,
            language,
            node.start_line,
        ));
        nodes.push(node);
    }
}

fn add_tsx_jsx_components(
    file_path: &str,
    language: Language,
    now: i64,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
) {
    if !matches!(language, Language::Tsx | Language::Jsx) {
        return;
    }
    let component_names: Vec<(String, i64, bool, Option<String>)> = nodes
        .iter()
        .filter(|node| matches!(node.kind, NodeKind::Function | NodeKind::Class))
        .filter(|node| node.name.chars().next().is_some_and(char::is_uppercase))
        .map(|node| {
            (
                node.name.clone(),
                node.start_line,
                node.is_exported,
                node.signature.clone(),
            )
        })
        .collect();
    for (name, line, is_exported, signature) in component_names {
        let mut node = make_node(
            file_path,
            language,
            NodeKind::Component,
            &name,
            line,
            0,
            now,
            signature,
        );
        node.is_exported = is_exported;
        node.visibility = node.is_exported.then(|| "public".to_string());
        add_contains(nodes, edges, &node);
        nodes.push(node);
    }
}

fn extract_python(
    file_path: &str,
    source: &str,
    now: i64,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
    refs: &mut Vec<UnresolvedReference>,
) {
    let class_re =
        Regex::new(r"^([ \t]*)class\s+([A-Za-z_][A-Za-z0-9_]*)(?:\s*\([^)]*\))?\s*:").unwrap();
    let def_re = Regex::new(
        r"^([ \t]*)(async\s+)?def\s+([A-Za-z_][A-Za-z0-9_]*)\s*\([^)]*\)\s*(?:->\s*[^:]+)?\s*:",
    )
    .unwrap();
    let decorator_re = Regex::new(r"^([ \t]*)@([A-Za-z_][A-Za-z0-9_\.]*)").unwrap();

    let mut class_stack: Vec<(usize, String, String)> = Vec::new();
    let mut pending_decorators: Vec<(usize, String, i64)> = Vec::new();

    for (line_idx, line) in source.lines().enumerate() {
        let line_no = line_idx as i64 + 1;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let indent = python_indent_width(line);
        while class_stack.last().is_some_and(|(class_indent, _, _)| {
            indent <= *class_indent && !trimmed.starts_with('@')
        }) {
            class_stack.pop();
        }
        pending_decorators.retain(|(decorator_indent, _, _)| *decorator_indent == indent);

        if let Some(cap) = decorator_re.captures(line) {
            pending_decorators.push((indent, cap[2].to_string(), line_no));
            continue;
        }

        if let Some(cap) = class_re.captures(line) {
            let name = cap[2].to_string();
            let node = make_node(
                file_path,
                Language::Python,
                NodeKind::Class,
                &name,
                line_no,
                indent as i64,
                now,
                Some(trimmed.to_string()),
            );
            add_contains(nodes, edges, &node);
            class_stack.push((indent, name, node.id.clone()));
            nodes.push(node);
            pending_decorators.clear();
            continue;
        }

        if let Some(cap) = def_re.captures(line) {
            let name = cap[3].to_string();
            let parent_class = class_stack
                .iter()
                .rev()
                .find(|(class_indent, _, _)| indent > *class_indent);
            let kind = if parent_class.is_some() {
                NodeKind::Method
            } else {
                NodeKind::Function
            };
            let mut signature_lines: Vec<String> = pending_decorators
                .iter()
                .map(|(_, decorator, _)| format!("@{}", decorator))
                .collect();
            signature_lines.push(trimmed.to_string());
            let mut node = make_node(
                file_path,
                Language::Python,
                kind,
                &name,
                line_no,
                indent as i64,
                now,
                Some(signature_lines.join("\n")),
            );
            node.is_async = cap.get(2).is_some();
            node.is_static = pending_decorators
                .iter()
                .any(|(_, decorator, _)| decorator == "staticmethod");
            if let Some((_, class_name, class_id)) = parent_class {
                node.qualified_name = format!("{}.{}", class_name, name);
                edges.push(Edge {
                    id: None,
                    source: class_id.clone(),
                    target: node.id.clone(),
                    kind: EdgeKind::Contains,
                    line: None,
                    col: None,
                    provenance: Some("python".into()),
                });
            } else {
                add_contains(nodes, edges, &node);
            }

            for (_, decorator, decorator_line) in &pending_decorators {
                refs_push(
                    refs,
                    &node.id,
                    decorator,
                    EdgeKind::Decorates,
                    file_path,
                    Language::Python,
                    *decorator_line,
                    0,
                );
            }
            nodes.push(node);
            pending_decorators.clear();
            continue;
        }

        extract_python_imports(file_path, line, line_no, now, nodes, edges, refs);
        pending_decorators.clear();
    }

    add_call_refs(
        file_path,
        source,
        Language::Python,
        nodes,
        refs,
        r"([A-Za-z_][A-Za-z0-9_\.]*)\s*\(",
    );
}

fn extract_python_imports(
    file_path: &str,
    line: &str,
    line_no: i64,
    now: i64,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
    refs: &mut Vec<UnresolvedReference>,
) {
    let import_re = Regex::new(r"^\s*import\s+(.+)$").unwrap();
    let from_re = Regex::new(r"^\s*from\s+([A-Za-z_\.][A-Za-z0-9_\.]*)\s+import\s+(.+)$").unwrap();
    let Some(file_id) = nodes.first().map(|node| node.id.clone()) else {
        return;
    };

    if let Some(cap) = import_re.captures(line) {
        for spec in cap[1]
            .split(',')
            .map(str::trim)
            .filter(|spec| !spec.is_empty())
        {
            let module = spec.split_whitespace().next().unwrap_or(spec);
            add_python_import_node(
                file_path,
                module,
                line.trim(),
                line_no,
                now,
                nodes,
                edges,
                refs,
                &file_id,
            );
        }
        return;
    }

    if let Some(cap) = from_re.captures(line) {
        let module = cap[1].trim();
        add_python_import_node(
            file_path,
            module,
            line.trim(),
            line_no,
            now,
            nodes,
            edges,
            refs,
            &file_id,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn add_python_import_node(
    file_path: &str,
    module: &str,
    signature: &str,
    line_no: i64,
    now: i64,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
    refs: &mut Vec<UnresolvedReference>,
    file_id: &str,
) {
    let node = make_node(
        file_path,
        Language::Python,
        NodeKind::Import,
        module,
        line_no,
        0,
        now,
        Some(signature.to_string()),
    );
    edges.push(Edge {
        id: None,
        source: file_id.to_string(),
        target: node.id.clone(),
        kind: EdgeKind::Contains,
        line: None,
        col: None,
        provenance: Some("python".into()),
    });
    refs_push(
        refs,
        file_id,
        module,
        EdgeKind::Imports,
        file_path,
        Language::Python,
        line_no,
        0,
    );
    nodes.push(node);
}

fn python_indent_width(line: &str) -> usize {
    line.chars()
        .take_while(|ch| matches!(ch, ' ' | '\t'))
        .map(|ch| if ch == '\t' { 4 } else { 1 })
        .sum()
}

fn extract_go(
    file_path: &str,
    source: &str,
    now: i64,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
    refs: &mut Vec<UnresolvedReference>,
) {
    let package_re = Regex::new(r"(?m)^\s*package\s+([A-Za-z_][A-Za-z0-9_]*)").unwrap();
    if let Some(cap) = package_re.captures(source) {
        let package = cap.get(1).unwrap();
        let node = make_node(
            file_path,
            Language::Go,
            NodeKind::Module,
            package.as_str(),
            line_for(source, package.start()),
            0,
            now,
            cap.get(0).map(|m| m.as_str().trim().to_string()),
        );
        add_contains(nodes, edges, &node);
        nodes.push(node);
    }

    add_go_imports(file_path, source, now, nodes, edges, refs);
    add_go_types(file_path, source, now, nodes, edges);
    add_go_functions(file_path, source, now, nodes, edges);
    add_go_call_refs(file_path, source, nodes, refs);
}

fn add_go_functions(
    file_path: &str,
    source: &str,
    now: i64,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
) {
    let method_re = Regex::new(
        r"(?m)^\s*func\s+\(\s*(?:[A-Za-z_][A-Za-z0-9_]*\s+)?\*?\s*([A-Za-z_][A-Za-z0-9_]*)\s*\)\s*([A-Za-z_][A-Za-z0-9_]*)\s*(\([^)]*\)\s*(?:\([^)]*\)|[A-Za-z_][A-Za-z0-9_\.\[\]]*)?)",
    )
    .unwrap();
    for cap in method_re.captures_iter(source) {
        let receiver = cap.get(1).unwrap().as_str();
        let name = cap.get(2).unwrap().as_str();
        let signature = cap.get(0).map(|m| m.as_str().trim().to_string());
        let mut node = make_node(
            file_path,
            Language::Go,
            NodeKind::Method,
            name,
            line_for(source, cap.get(2).unwrap().start()),
            0,
            now,
            signature,
        );
        node.qualified_name = format!("{}.{}", receiver, name);
        add_contains(nodes, edges, &node);
        nodes.push(node);
    }

    let function_re = Regex::new(
        r"(?m)^\s*func\s+([A-Za-z_][A-Za-z0-9_]*)\s*(\([^)]*\)\s*(?:\([^)]*\)|[A-Za-z_][A-Za-z0-9_\.\[\]]*)?)",
    )
    .unwrap();
    for cap in function_re.captures_iter(source) {
        let name = cap.get(1).unwrap().as_str();
        let signature = cap.get(0).map(|m| m.as_str().trim().to_string());
        let node = make_node(
            file_path,
            Language::Go,
            NodeKind::Function,
            name,
            line_for(source, cap.get(1).unwrap().start()),
            0,
            now,
            signature,
        );
        add_contains(nodes, edges, &node);
        nodes.push(node);
    }
}

fn add_go_types(
    file_path: &str,
    source: &str,
    now: i64,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
) {
    let type_re =
        Regex::new(r"(?m)^\s*type\s+([A-Za-z_][A-Za-z0-9_]*)\s+(struct|interface)\s*\{").unwrap();
    for cap in type_re.captures_iter(source) {
        let kind = match cap.get(2).unwrap().as_str() {
            "struct" => NodeKind::Struct,
            "interface" => NodeKind::Interface,
            _ => continue,
        };
        let name = cap.get(1).unwrap();
        let node = make_node(
            file_path,
            Language::Go,
            kind,
            name.as_str(),
            line_for(source, name.start()),
            0,
            now,
            cap.get(0).map(|m| m.as_str().trim().to_string()),
        );
        add_contains(nodes, edges, &node);
        nodes.push(node);
    }
}

fn add_go_imports(
    file_path: &str,
    source: &str,
    now: i64,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
    refs: &mut Vec<UnresolvedReference>,
) {
    let single_re =
        Regex::new(r#"(?m)^\s*import\s+(?:(\.|_|[A-Za-z_][A-Za-z0-9_]*)\s+)?"([^"]+)""#).unwrap();
    for cap in single_re.captures_iter(source) {
        let module = cap.get(2).unwrap();
        add_go_import_node(
            file_path,
            module.as_str(),
            cap.get(0).unwrap().as_str().trim(),
            line_for(source, module.start()),
            now,
            nodes,
            edges,
            refs,
        );
    }

    let block_re = Regex::new(r#"(?ms)^\s*import\s*\((?P<body>.*?)\)"#).unwrap();
    let item_re = Regex::new(r#"(?m)^\s*(?:(\.|_|[A-Za-z_][A-Za-z0-9_]*)\s+)?"([^"]+)""#).unwrap();
    for block in block_re.captures_iter(source) {
        let Some(body) = block.name("body") else {
            continue;
        };
        for cap in item_re.captures_iter(body.as_str()) {
            let module = cap.get(2).unwrap();
            let absolute_module_start = body.start() + module.start();
            add_go_import_node(
                file_path,
                module.as_str(),
                cap.get(0).unwrap().as_str().trim(),
                line_for(source, absolute_module_start),
                now,
                nodes,
                edges,
                refs,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn add_go_import_node(
    file_path: &str,
    module: &str,
    signature: &str,
    line: i64,
    now: i64,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
    refs: &mut Vec<UnresolvedReference>,
) {
    let Some(file_id) = nodes.first().map(|node| node.id.clone()) else {
        return;
    };
    let node = make_node(
        file_path,
        Language::Go,
        NodeKind::Import,
        module,
        line,
        0,
        now,
        Some(signature.to_string()),
    );
    add_contains(nodes, edges, &node);
    refs_push(
        refs,
        &file_id,
        module,
        EdgeKind::Imports,
        file_path,
        Language::Go,
        line,
        0,
    );
    nodes.push(node);
}

fn add_go_call_refs(
    file_path: &str,
    source: &str,
    nodes: &[Node],
    refs: &mut Vec<UnresolvedReference>,
) {
    let call_re = Regex::new(r"([A-Za-z_][A-Za-z0-9_\.]*)\s*\(").unwrap();
    let keywords = [
        "append", "cap", "close", "complex", "copy", "delete", "func", "if", "imag", "len", "make",
        "new", "panic", "print", "println", "real", "recover", "return", "switch",
    ];
    for cap in call_re.captures_iter(source) {
        let name_match = cap.get(1).unwrap();
        let name = name_match.as_str();
        let line = line_for(source, name_match.start());
        let line_text = source
            .lines()
            .nth(line.saturating_sub(1) as usize)
            .unwrap_or_default()
            .trim_start();
        if keywords.contains(&name)
            || line_text.starts_with("func ")
            || line_text.starts_with("type ")
        {
            continue;
        }
        if let Some(caller) = nodes
            .iter()
            .filter(|n| matches!(n.kind, NodeKind::Function | NodeKind::Method))
            .rev()
            .find(|n| n.start_line <= line)
        {
            refs_push(
                refs,
                &caller.id,
                name,
                EdgeKind::Calls,
                file_path,
                Language::Go,
                line,
                0,
            );
        }
    }
}

fn extract_java_kotlin(
    file_path: &str,
    source: &str,
    language: Language,
    now: i64,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
    refs: &mut Vec<UnresolvedReference>,
) {
    add_java_kotlin_imports(file_path, source, language, now, nodes, edges, refs);
    add_java_kotlin_types_and_members(file_path, source, language, now, nodes, edges, refs);
    add_call_refs(
        file_path,
        source,
        language,
        nodes,
        refs,
        r"([A-Za-z_][A-Za-z0-9_\.]*)\s*\(",
    );
}

fn add_java_kotlin_imports(
    file_path: &str,
    source: &str,
    language: Language,
    now: i64,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
    refs: &mut Vec<UnresolvedReference>,
) {
    let re =
        Regex::new(r"(?m)^\s*import\s+(?:static\s+)?([A-Za-z_][A-Za-z0-9_\.\*]*)\s*;?").unwrap();
    for cap in re.captures_iter(source) {
        let module = cap.get(1).unwrap();
        let node = make_node(
            file_path,
            language,
            NodeKind::Import,
            module.as_str(),
            line_for(source, module.start()),
            0,
            now,
            cap.get(0).map(|m| m.as_str().trim().to_string()),
        );
        add_contains(nodes, edges, &node);
        refs_push(
            refs,
            &nodes[0].id,
            module.as_str(),
            EdgeKind::Imports,
            file_path,
            language,
            node.start_line,
            0,
        );
        nodes.push(node);
    }
}

fn add_java_kotlin_types_and_members(
    file_path: &str,
    source: &str,
    language: Language,
    now: i64,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
    refs: &mut Vec<UnresolvedReference>,
) {
    let type_re = Regex::new(
        r"^\s*(?:(public|private|protected|internal)\s+)?(?:(?:data|abstract|open|final|sealed|enum)\s+)*(class|interface|enum)\s+([A-Za-z_][A-Za-z0-9_]*)([^{]*)\{?",
    )
    .unwrap();
    let java_method_re = Regex::new(
        r"^\s*(?:(public|private|protected)\s+)?((?:static|final|abstract|synchronized)\s+)*(?:[A-Za-z_][A-Za-z0-9_<>,\.\?\[\]\s]*\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*\([^;{}]*\)\s*(?:throws\s+[A-Za-z0-9_,\.\s]+)?\{?",
    )
    .unwrap();
    let kotlin_fun_re = Regex::new(
        r"^\s*(?:(public|private|protected|internal)\s+)?((?:suspend|inline|open|override|private|public|protected|internal)\s+)*fun\s+(?:(?P<receiver>[A-Za-z_][A-Za-z0-9_\.]*)\.)?(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s*\([^)]*\)\s*(?::\s*[A-Za-z_][A-Za-z0-9_<>,\.\?\s]*)?",
    )
    .unwrap();
    let annotation_re = Regex::new(r"^\s*@([A-Za-z_][A-Za-z0-9_\.]*)").unwrap();
    let mut type_stack: Vec<(usize, String, String)> = Vec::new();
    let mut pending_annotations: Vec<(String, i64)> = Vec::new();

    for (idx, line) in source.lines().enumerate() {
        let line_no = idx as i64 + 1;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }
        let indent = python_indent_width(line);
        while type_stack
            .last()
            .is_some_and(|(type_indent, _, _)| indent <= *type_indent && trimmed.starts_with('}'))
        {
            type_stack.pop();
        }

        if let Some(cap) = annotation_re.captures(line) {
            pending_annotations.push((cap[1].to_string(), line_no));
            continue;
        }

        if let Some(cap) = type_re.captures(line) {
            let keyword = cap.get(2).unwrap().as_str();
            let kind = match keyword {
                "interface" => NodeKind::Interface,
                "enum" => NodeKind::Enum,
                _ => NodeKind::Class,
            };
            let name = cap.get(3).unwrap();
            let mut node = make_node(
                file_path,
                language,
                kind,
                name.as_str(),
                line_no,
                indent as i64,
                now,
                Some(java_kotlin_signature(&pending_annotations, trimmed)),
            );
            node.visibility = cap.get(1).map(|m| m.as_str().to_string());
            node.is_exported = node
                .visibility
                .as_deref()
                .map(|visibility| visibility == "public")
                .unwrap_or(language == Language::Kotlin);
            add_contains(nodes, edges, &node);
            add_java_kotlin_metadata_refs(
                &node.id,
                cap.get(4).map(|m| m.as_str()).unwrap_or_default(),
                &pending_annotations,
                file_path,
                language,
                line_no,
                refs,
            );
            type_stack.push((indent, name.as_str().to_string(), node.id.clone()));
            nodes.push(node);
            pending_annotations.clear();
            continue;
        }

        let member = match language {
            Language::Kotlin => kotlin_fun_re.captures(line).map(|cap| {
                (
                    cap.name("name").unwrap().as_str().to_string(),
                    cap.name("receiver").map(|m| m.as_str().to_string()),
                    cap.get(1).map(|m| m.as_str().to_string()),
                    cap.get(2)
                        .map(|m| m.as_str().contains("suspend"))
                        .unwrap_or(false),
                    cap.get(0).unwrap().as_str().trim().to_string(),
                )
            }),
            _ => java_method_re.captures(line).and_then(|cap| {
                let name = cap.get(3).unwrap().as_str();
                let skip = matches!(
                    name,
                    "if" | "for" | "while" | "switch" | "catch" | "return" | "new"
                );
                (!skip).then(|| {
                    (
                        name.to_string(),
                        None,
                        cap.get(1).map(|m| m.as_str().to_string()),
                        false,
                        cap.get(0).unwrap().as_str().trim().to_string(),
                    )
                })
            }),
        };
        if let Some((name, receiver, visibility, is_async, signature)) = member {
            let kind = if type_stack.is_empty() && language == Language::Kotlin {
                NodeKind::Function
            } else {
                NodeKind::Method
            };
            let mut node = make_node(
                file_path,
                language,
                kind,
                &name,
                line_no,
                indent as i64,
                now,
                Some(java_kotlin_signature(&pending_annotations, &signature)),
            );
            node.visibility = visibility;
            node.is_exported = node
                .visibility
                .as_deref()
                .map(|visibility| visibility == "public")
                .unwrap_or(language == Language::Kotlin);
            node.is_async = is_async;
            node.is_static = signature.contains(" static ");
            if let Some(receiver) =
                receiver.or_else(|| type_stack.last().map(|(_, name, _)| name.clone()))
            {
                node.qualified_name = format!("{}.{}", receiver, name);
            }
            if let Some((_, _, parent_id)) = type_stack.last() {
                edges.push(Edge {
                    id: None,
                    source: parent_id.clone(),
                    target: node.id.clone(),
                    kind: EdgeKind::Contains,
                    line: None,
                    col: None,
                    provenance: Some(language.as_str().into()),
                });
            } else {
                add_contains(nodes, edges, &node);
            }
            for (annotation, annotation_line) in &pending_annotations {
                refs_push(
                    refs,
                    &node.id,
                    annotation,
                    EdgeKind::Decorates,
                    file_path,
                    language,
                    *annotation_line,
                    0,
                );
            }
            nodes.push(node);
            pending_annotations.clear();
            continue;
        }

        pending_annotations.clear();
    }
}

fn java_kotlin_signature(annotations: &[(String, i64)], declaration: &str) -> String {
    if annotations.is_empty() {
        declaration.to_string()
    } else {
        let mut lines: Vec<String> = annotations
            .iter()
            .map(|(annotation, _)| format!("@{}", annotation))
            .collect();
        lines.push(declaration.to_string());
        lines.join("\n")
    }
}

#[allow(clippy::too_many_arguments)]
fn add_java_kotlin_metadata_refs(
    node_id: &str,
    tail: &str,
    annotations: &[(String, i64)],
    file_path: &str,
    language: Language,
    line: i64,
    refs: &mut Vec<UnresolvedReference>,
) {
    let extends_re = Regex::new(r"\bextends\s+([A-Za-z_][A-Za-z0-9_\.]*)").unwrap();
    let implements_re = Regex::new(r"\bimplements\s+([A-Za-z_][A-Za-z0-9_\.,\s]*)").unwrap();
    let kotlin_super_re = Regex::new(r":\s*([A-Za-z_][A-Za-z0-9_\.]*)").unwrap();
    if let Some(cap) = extends_re
        .captures(tail)
        .or_else(|| kotlin_super_re.captures(tail))
    {
        refs_push(
            refs,
            node_id,
            cap.get(1).unwrap().as_str(),
            EdgeKind::Extends,
            file_path,
            language,
            line,
            0,
        );
    }
    if let Some(cap) = implements_re.captures(tail) {
        for name in cap[1]
            .split(',')
            .map(str::trim)
            .filter(|name| !name.is_empty())
        {
            refs_push(
                refs,
                node_id,
                name,
                EdgeKind::Implements,
                file_path,
                language,
                line,
                0,
            );
        }
    }
    for (annotation, annotation_line) in annotations {
        refs_push(
            refs,
            node_id,
            annotation,
            EdgeKind::Decorates,
            file_path,
            language,
            *annotation_line,
            0,
        );
    }
}

fn extract_csharp(
    file_path: &str,
    source: &str,
    now: i64,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
    refs: &mut Vec<UnresolvedReference>,
) {
    add_csharp_usings(file_path, source, now, nodes, edges, refs);
    add_csharp_types_and_members(file_path, source, now, nodes, edges, refs);
    add_csharp_call_refs(file_path, source, nodes, refs);
}

fn add_csharp_usings(
    file_path: &str,
    source: &str,
    now: i64,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
    refs: &mut Vec<UnresolvedReference>,
) {
    let re = Regex::new(
        r"(?m)^\s*using\s+(?:static\s+)?(?:(?:[A-Za-z_][A-Za-z0-9_]*)\s*=\s*)?([A-Za-z_][A-Za-z0-9_\.]*)\s*;",
    )
    .unwrap();
    for cap in re.captures_iter(source) {
        let module = cap.get(1).unwrap();
        let node = make_node(
            file_path,
            Language::CSharp,
            NodeKind::Import,
            module.as_str(),
            line_for(source, module.start()),
            0,
            now,
            cap.get(0).map(|m| m.as_str().trim().to_string()),
        );
        add_contains(nodes, edges, &node);
        refs_push(
            refs,
            &nodes[0].id,
            module.as_str(),
            EdgeKind::Imports,
            file_path,
            Language::CSharp,
            node.start_line,
            0,
        );
        nodes.push(node);
    }
}

fn add_csharp_types_and_members(
    file_path: &str,
    source: &str,
    now: i64,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
    refs: &mut Vec<UnresolvedReference>,
) {
    let type_re = Regex::new(
        r"^\s*(?:(public|private|protected|internal)\s+)?(?:(?:abstract|sealed|static|partial)\s+)*(class|interface|struct|enum)\s+([A-Za-z_][A-Za-z0-9_]*)([^{]*)\{?",
    )
    .unwrap();
    let method_re = Regex::new(
        r"^\s*(?:(public|private|protected|internal)\s+)?((?:static|async|virtual|override|abstract|sealed|partial)\s+)*(?:[A-Za-z_][A-Za-z0-9_<>,\.\?\[\]\s]*\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*\([^;{}]*\)\s*(?:where\s+[^{]+)?\{?",
    )
    .unwrap();
    let property_re = Regex::new(
        r"^\s*(?:(public|private|protected|internal)\s+)?((?:static|virtual|override|abstract|sealed)\s+)*(?:[A-Za-z_][A-Za-z0-9_<>,\.\?\[\]\s]*\s+)([A-Za-z_][A-Za-z0-9_]*)\s*\{",
    )
    .unwrap();
    let attribute_re = Regex::new(r"^\s*\[\s*([A-Za-z_][A-Za-z0-9_\.]*)").unwrap();
    let mut type_stack: Vec<(usize, String, String)> = Vec::new();
    let mut pending_attributes: Vec<(String, i64)> = Vec::new();

    for (idx, line) in source.lines().enumerate() {
        let line_no = idx as i64 + 1;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }
        let indent = python_indent_width(line);
        while type_stack
            .last()
            .is_some_and(|(type_indent, _, _)| indent <= *type_indent && trimmed.starts_with('}'))
        {
            type_stack.pop();
        }

        if let Some(cap) = attribute_re.captures(line) {
            pending_attributes.push((cap[1].to_string(), line_no));
            continue;
        }

        if let Some(cap) = type_re.captures(line) {
            let keyword = cap.get(2).unwrap().as_str();
            let kind = match keyword {
                "interface" => NodeKind::Interface,
                "struct" => NodeKind::Struct,
                "enum" => NodeKind::Enum,
                _ => NodeKind::Class,
            };
            let name = cap.get(3).unwrap();
            let mut node = make_node(
                file_path,
                Language::CSharp,
                kind,
                name.as_str(),
                line_no,
                indent as i64,
                now,
                Some(csharp_signature(&pending_attributes, trimmed)),
            );
            node.visibility = cap
                .get(1)
                .map(|m| m.as_str().to_string())
                .or_else(|| Some("private".to_string()));
            node.is_exported = node.visibility.as_deref() == Some("public");
            add_contains(nodes, edges, &node);
            add_csharp_metadata_refs(
                &node.id,
                cap.get(4).map(|m| m.as_str()).unwrap_or_default(),
                &pending_attributes,
                file_path,
                line_no,
                refs,
            );
            type_stack.push((indent, name.as_str().to_string(), node.id.clone()));
            nodes.push(node);
            pending_attributes.clear();
            continue;
        }

        let member = method_re
            .captures(line)
            .and_then(|cap| {
                let name = cap.get(3).unwrap().as_str();
                let skip = matches!(
                    name,
                    "if" | "for" | "foreach" | "while" | "switch" | "catch" | "return" | "new"
                );
                (!skip).then(|| {
                    (
                        NodeKind::Method,
                        name.to_string(),
                        cap.get(1).map(|m| m.as_str().to_string()),
                        cap.get(2)
                            .map(|m| m.as_str().to_string())
                            .unwrap_or_default(),
                        cap.get(0).unwrap().as_str().trim().to_string(),
                    )
                })
            })
            .or_else(|| {
                property_re.captures(line).map(|cap| {
                    (
                        NodeKind::Property,
                        cap.get(3).unwrap().as_str().to_string(),
                        cap.get(1).map(|m| m.as_str().to_string()),
                        cap.get(2)
                            .map(|m| m.as_str().to_string())
                            .unwrap_or_default(),
                        cap.get(0).unwrap().as_str().trim().to_string(),
                    )
                })
            });

        if let Some((kind, name, visibility, modifiers, signature)) = member {
            let mut node = make_node(
                file_path,
                Language::CSharp,
                kind,
                &name,
                line_no,
                indent as i64,
                now,
                Some(csharp_signature(&pending_attributes, &signature)),
            );
            node.visibility = visibility.or_else(|| Some("private".to_string()));
            node.is_exported = node.visibility.as_deref() == Some("public");
            node.is_static = modifiers.contains("static") || signature.contains(" static ");
            node.is_async = modifiers.contains("async") || signature.contains(" async ");
            if let Some((_, type_name, parent_id)) = type_stack.last() {
                node.qualified_name = format!("{}.{}", type_name, name);
                edges.push(Edge {
                    id: None,
                    source: parent_id.clone(),
                    target: node.id.clone(),
                    kind: EdgeKind::Contains,
                    line: None,
                    col: None,
                    provenance: Some("csharp".into()),
                });
            } else {
                add_contains(nodes, edges, &node);
            }
            for (attribute, attribute_line) in &pending_attributes {
                refs_push(
                    refs,
                    &node.id,
                    attribute,
                    EdgeKind::Decorates,
                    file_path,
                    Language::CSharp,
                    *attribute_line,
                    0,
                );
            }
            nodes.push(node);
            pending_attributes.clear();
            continue;
        }

        pending_attributes.clear();
    }
}

fn csharp_signature(attributes: &[(String, i64)], declaration: &str) -> String {
    if attributes.is_empty() {
        declaration.to_string()
    } else {
        let mut lines: Vec<String> = attributes
            .iter()
            .map(|(attribute, _)| format!("[{}]", attribute))
            .collect();
        lines.push(declaration.to_string());
        lines.join("\n")
    }
}

fn add_csharp_metadata_refs(
    node_id: &str,
    tail: &str,
    attributes: &[(String, i64)],
    file_path: &str,
    line: i64,
    refs: &mut Vec<UnresolvedReference>,
) {
    let base_tail = tail.trim().strip_prefix(':').unwrap_or("").trim();
    let mut bases = base_tail
        .split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(|name| name.split_whitespace().next().unwrap_or(name));
    if let Some(base) = bases.next() {
        refs_push(
            refs,
            node_id,
            base,
            EdgeKind::Extends,
            file_path,
            Language::CSharp,
            line,
            0,
        );
    }
    for name in bases {
        refs_push(
            refs,
            node_id,
            name,
            EdgeKind::Implements,
            file_path,
            Language::CSharp,
            line,
            0,
        );
    }
    for (attribute, attribute_line) in attributes {
        refs_push(
            refs,
            node_id,
            attribute,
            EdgeKind::Decorates,
            file_path,
            Language::CSharp,
            *attribute_line,
            0,
        );
    }
}

fn add_csharp_call_refs(
    file_path: &str,
    source: &str,
    nodes: &[Node],
    refs: &mut Vec<UnresolvedReference>,
) {
    let re = Regex::new(r"([A-Za-z_][A-Za-z0-9_\.]*)\s*(?:<[^;\n()]+>)?\s*\(").unwrap();
    let keywords = [
        "if", "for", "foreach", "while", "switch", "catch", "return", "new", "typeof", "nameof",
        "using",
    ];
    for cap in re.captures_iter(source) {
        let name_match = cap.get(1).unwrap();
        let name = name_match.as_str();
        let line = line_for(source, name_match.start());
        let line_text = source
            .lines()
            .nth(line.saturating_sub(1) as usize)
            .unwrap_or_default()
            .trim_start();
        if keywords.contains(&name)
            || line_text.contains(&format!("{name}("))
                && matches!(
                    line_text.split_whitespace().next(),
                    Some("public" | "private" | "protected" | "internal" | "static" | "async")
                )
        {
            continue;
        }
        if let Some(caller) = nodes
            .iter()
            .filter(|n| matches!(n.kind, NodeKind::Function | NodeKind::Method))
            .rev()
            .find(|n| n.start_line <= line)
        {
            refs_push(
                refs,
                &caller.id,
                name,
                EdgeKind::Calls,
                file_path,
                Language::CSharp,
                line,
                0,
            );
        }
    }
}

fn extract_rust(
    file_path: &str,
    source: &str,
    now: i64,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
    refs: &mut Vec<UnresolvedReference>,
) {
    if try_extract_rust_tree_sitter(file_path, source, now, nodes, edges, refs) {
        return;
    }

    add_regex_nodes(
        file_path,
        source,
        Language::Rust,
        now,
        nodes,
        edges,
        r"(?m)^\s*(pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)\s*([^{;]*)",
        NodeKind::Function,
    );
    add_regex_nodes(
        file_path,
        source,
        Language::Rust,
        now,
        nodes,
        edges,
        r"(?m)^\s*(pub(?:\([^)]*\))?\s+)?struct\s+([A-Za-z_][A-Za-z0-9_]*)",
        NodeKind::Struct,
    );
    add_regex_nodes(
        file_path,
        source,
        Language::Rust,
        now,
        nodes,
        edges,
        r"(?m)^\s*(pub(?:\([^)]*\))?\s+)?trait\s+([A-Za-z_][A-Za-z0-9_]*)",
        NodeKind::Trait,
    );
    add_regex_nodes(
        file_path,
        source,
        Language::Rust,
        now,
        nodes,
        edges,
        r"(?m)^\s*(pub(?:\([^)]*\))?\s+)?enum\s+([A-Za-z_][A-Za-z0-9_]*)",
        NodeKind::Enum,
    );
    add_regex_nodes(
        file_path,
        source,
        Language::Rust,
        now,
        nodes,
        edges,
        r"(?m)^\s*(pub(?:\([^)]*\))?\s+)?type\s+([A-Za-z_][A-Za-z0-9_]*)",
        NodeKind::TypeAlias,
    );

    let use_re = Regex::new(r"(?m)^\s*use\s+([^;]+);").unwrap();
    for cap in use_re.captures_iter(source) {
        let full = cap.get(1).unwrap();
        let root = full
            .as_str()
            .split("::")
            .next()
            .unwrap_or(full.as_str())
            .trim_matches('{')
            .trim();
        let node = make_node(
            file_path,
            Language::Rust,
            NodeKind::Import,
            root,
            line_for(source, full.start()),
            0,
            now,
            Some(format!("use {};", full.as_str())),
        );
        add_contains(nodes, edges, &node);
        refs.push(unresolved(
            &nodes[0].id,
            root,
            EdgeKind::Imports,
            file_path,
            Language::Rust,
            node.start_line,
        ));
        nodes.push(node);
    }

    let impl_re = Regex::new(
        r"(?m)^\s*impl(?:<[^>]+>)?\s+([A-Za-z_][A-Za-z0-9_:]*)\s+for\s+([A-Za-z_][A-Za-z0-9_]*)",
    )
    .unwrap();
    for cap in impl_re.captures_iter(source) {
        let trait_name = cap.get(1).unwrap().as_str().rsplit("::").next().unwrap();
        let type_name = cap.get(2).unwrap().as_str();
        if let Some(src) = nodes
            .iter()
            .find(|n| n.name == type_name && matches!(n.kind, NodeKind::Struct | NodeKind::Enum))
            .map(|n| n.id.clone())
        {
            refs.push(unresolved(
                &src,
                trait_name,
                EdgeKind::Implements,
                file_path,
                Language::Rust,
                line_for(source, cap.get(1).unwrap().start()),
            ));
        }
    }
    add_call_refs(
        file_path,
        source,
        Language::Rust,
        nodes,
        refs,
        r"([A-Za-z_][A-Za-z0-9_:]*)\s*\(",
    );
}

fn extract_moonbit(
    file_path: &str,
    source: &str,
    now: i64,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
    refs: &mut Vec<UnresolvedReference>,
) {
    if file_path.ends_with("moon.mod.json")
        || file_path.ends_with("moon.pkg.json")
        || file_path.ends_with("moon.pkg")
    {
        extract_moonbit_metadata(file_path, source, now, nodes, edges, refs);
        return;
    }

    let source = if file_path.ends_with(".mbt.md") {
        extract_mbt_markdown_code_with_padding(source)
    } else {
        source.to_string()
    };

    if try_extract_moonbit_tree_sitter(file_path, &source, now, nodes, edges, refs) {
        extract_moonbit_sol_routes(file_path, &source, now, nodes, edges, refs);
        return;
    }

    add_regex_nodes(
        file_path,
        &source,
        Language::MoonBit,
        now,
        nodes,
        edges,
        r"(?m)^\s*(pub\s+)?(?:async\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)\s*([^{]*)",
        NodeKind::Function,
    );
    add_regex_nodes(
        file_path,
        &source,
        Language::MoonBit,
        now,
        nodes,
        edges,
        r"(?m)^\s*(pub\s+)?(?:async\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*::[A-Za-z_][A-Za-z0-9_]*)\s*([^{]*)",
        NodeKind::Method,
    );
    add_regex_nodes(
        file_path,
        &source,
        Language::MoonBit,
        now,
        nodes,
        edges,
        r"(?m)^\s*(pub\s+)?struct\s+([A-Za-z_][A-Za-z0-9_]*)",
        NodeKind::Struct,
    );
    add_regex_nodes(
        file_path,
        &source,
        Language::MoonBit,
        now,
        nodes,
        edges,
        r"(?m)^\s*(pub\s+)?trait\s+([A-Za-z_][A-Za-z0-9_]*)",
        NodeKind::Trait,
    );
    add_regex_nodes(
        file_path,
        &source,
        Language::MoonBit,
        now,
        nodes,
        edges,
        r"(?m)^\s*(pub\s+)?enum\s+([A-Za-z_][A-Za-z0-9_]*)",
        NodeKind::Enum,
    );
    add_regex_nodes(
        file_path,
        &source,
        Language::MoonBit,
        now,
        nodes,
        edges,
        r"(?m)^\s*(pub\s+)?type\s+([A-Za-z_][A-Za-z0-9_]*)",
        NodeKind::TypeAlias,
    );
    add_regex_nodes(
        file_path,
        &source,
        Language::MoonBit,
        now,
        nodes,
        edges,
        r"(?m)^\s*(pub\s+)?let\s+([A-Za-z_][A-Za-z0-9_]*)",
        NodeKind::Variable,
    );

    let import_re =
        Regex::new(r#"(?m)^\s*import\s+([@\w/.\-]+)(?:\s+as\s+([A-Za-z_][A-Za-z0-9_]*))?"#)
            .unwrap();
    for cap in import_re.captures_iter(&source) {
        let package = cap.get(1).unwrap().as_str();
        let name = cap.get(2).map(|m| m.as_str()).unwrap_or(package);
        let node = make_node(
            file_path,
            Language::MoonBit,
            NodeKind::Import,
            name,
            line_for(&source, cap.get(0).unwrap().start()),
            0,
            now,
            Some(cap.get(0).unwrap().as_str().to_string()),
        );
        add_contains(nodes, edges, &node);
        refs.push(unresolved(
            &nodes[0].id,
            name,
            EdgeKind::Imports,
            file_path,
            Language::MoonBit,
            node.start_line,
        ));
        nodes.push(node);
    }
    add_call_refs(
        file_path,
        &source,
        Language::MoonBit,
        nodes,
        refs,
        r"([@A-Za-z_][@A-Za-z0-9_:/]*)\s*\(",
    );
    extract_moonbit_sol_routes(file_path, &source, now, nodes, edges, refs);
}

fn extract_moonbit_sol_routes(
    file_path: &str,
    source: &str,
    now: i64,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
    refs: &mut Vec<UnresolvedReference>,
) {
    if !file_path.ends_with(".mbt") && !file_path.ends_with(".mbt.md") {
        return;
    }

    let safe = strip_moonbit_comments_preserve_lines(source);
    let call_re = Regex::new(
        r#"@(?:sol|router)\.(route|page|api_get|api_post|api_put|api_delete|api_patch|raw_get|raw_post|raw_put|raw_delete|raw_patch)\s*\(\s*"([^"]+)"\s*,\s*([@A-Za-z_][@A-Za-z0-9_:.]*)"#,
    )
    .unwrap();
    let wrap_re = Regex::new(r#"@(?:sol|router)\.wrap\s*\(\s*"([^"]*)"\s*,"#).unwrap();
    let constructor_re = Regex::new(
        r#"SolRoutes::(Page|RawGet|RawPost|RawPut|RawDelete|RawPatch)\s*\([^)]*path\s*=\s*"([^"]+)"[^)]*handler\s*=\s*(?:PageHandler|RawHandler)?\(?\s*([@A-Za-z_][@A-Za-z0-9_:.]*)"#,
    )
    .unwrap();
    let named_page_re = Regex::new(
        r#"@(?:sol|router)\.page\s*\([^)]*path\s*=\s*"([^"]+)"[^)]*handler\s*=\s*([@A-Za-z_][@A-Za-z0-9_:.]*)"#,
    )
    .unwrap();

    let mut prefix_stack: Vec<(usize, String)> = Vec::new();
    let mut byte_offset = 0usize;
    for line in safe.lines() {
        let indent = line.chars().take_while(|c| c.is_whitespace()).count();
        while prefix_stack
            .last()
            .map(|(stack_indent, _)| indent <= *stack_indent && line.trim_start().starts_with(']'))
            .unwrap_or(false)
        {
            prefix_stack.pop();
        }

        if let Some(cap) = wrap_re.captures(line) {
            let prefix = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let full_prefix = join_route_paths(current_route_prefix(&prefix_stack), prefix);
            prefix_stack.push((indent, full_prefix));
        }

        for cap in call_re.captures_iter(line) {
            let helper = cap.get(1).unwrap().as_str();
            let path = cap.get(2).unwrap().as_str();
            let handler = cap.get(3).map(|m| clean_moonbit_handler(m.as_str()));
            let route_path = join_route_paths(current_route_prefix(&prefix_stack), path);
            add_moonbit_route_node(
                file_path,
                &safe,
                byte_offset + cap.get(0).unwrap().start(),
                helper_route_method(helper),
                &route_path,
                handler.as_deref(),
                now,
                nodes,
                edges,
                refs,
            );
        }

        for cap in named_page_re.captures_iter(line) {
            let path = cap.get(1).unwrap().as_str();
            let handler = cap.get(2).map(|m| clean_moonbit_handler(m.as_str()));
            let route_path = join_route_paths(current_route_prefix(&prefix_stack), path);
            add_moonbit_route_node(
                file_path,
                &safe,
                byte_offset + cap.get(0).unwrap().start(),
                "PAGE",
                &route_path,
                handler.as_deref(),
                now,
                nodes,
                edges,
                refs,
            );
        }

        for cap in constructor_re.captures_iter(line) {
            let variant = cap.get(1).unwrap().as_str();
            let path = cap.get(2).unwrap().as_str();
            let handler = cap.get(3).map(|m| clean_moonbit_handler(m.as_str()));
            let route_path = join_route_paths(current_route_prefix(&prefix_stack), path);
            add_moonbit_route_node(
                file_path,
                &safe,
                byte_offset + cap.get(0).unwrap().start(),
                constructor_route_method(variant),
                &route_path,
                handler.as_deref(),
                now,
                nodes,
                edges,
                refs,
            );
        }

        byte_offset += line.len() + 1;
    }
}

fn add_moonbit_route_node(
    file_path: &str,
    source: &str,
    byte_offset: usize,
    method: &str,
    route_path: &str,
    handler: Option<&str>,
    now: i64,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
    refs: &mut Vec<UnresolvedReference>,
) {
    let line = line_for(source, byte_offset);
    let name = format!("{method} {route_path}");
    let node = Node {
        id: format!("route:{file_path}:{line}:{method}:{route_path}"),
        kind: NodeKind::Route,
        name,
        qualified_name: format!("{file_path}::route:{method}:{route_path}"),
        file_path: file_path.to_string(),
        language: Language::MoonBit,
        start_line: line,
        end_line: line,
        start_column: 0,
        end_column: 0,
        docstring: None,
        signature: handler.map(|h| format!("{method} {route_path} -> {h}")),
        visibility: None,
        is_exported: false,
        is_async: false,
        is_static: false,
        is_abstract: false,
        updated_at: now,
    };
    add_contains(nodes, edges, &node);
    if let Some(handler) = handler {
        refs.push(unresolved(
            &node.id,
            handler,
            EdgeKind::References,
            file_path,
            Language::MoonBit,
            line,
        ));
    }
    nodes.push(node);
}

fn helper_route_method(helper: &str) -> &'static str {
    match helper {
        "route" | "page" => "PAGE",
        "api_get" => "GET",
        "api_post" => "POST",
        "api_put" => "PUT",
        "api_delete" => "DELETE",
        "api_patch" => "PATCH",
        "raw_get" => "RAW GET",
        "raw_post" => "RAW POST",
        "raw_put" => "RAW PUT",
        "raw_delete" => "RAW DELETE",
        "raw_patch" => "RAW PATCH",
        _ => "PAGE",
    }
}

fn constructor_route_method(variant: &str) -> &'static str {
    match variant {
        "RawGet" => "RAW GET",
        "RawPost" => "RAW POST",
        "RawPut" => "RAW PUT",
        "RawDelete" => "RAW DELETE",
        "RawPatch" => "RAW PATCH",
        _ => "PAGE",
    }
}

fn current_route_prefix(prefix_stack: &[(usize, String)]) -> &str {
    prefix_stack
        .last()
        .map(|(_, prefix)| prefix.as_str())
        .unwrap_or("")
}

fn join_route_paths(prefix: &str, path: &str) -> String {
    if prefix.is_empty() || prefix == "/" {
        return normalize_route_path(path);
    }
    let path = normalize_route_path(path);
    if path == "/" {
        return normalize_route_path(prefix);
    }
    format!(
        "{}/{}",
        prefix.trim_end_matches('/'),
        path.trim_start_matches('/')
    )
}

fn normalize_route_path(path: &str) -> String {
    if path.is_empty() {
        return "/".into();
    }
    let path = path.replace('\\', "/");
    if path.starts_with('/') {
        path
    } else {
        format!("/{path}")
    }
}

fn clean_moonbit_handler(handler: &str) -> String {
    handler
        .trim()
        .trim_start_matches('@')
        .rsplit(['.', ':'])
        .next()
        .unwrap_or(handler)
        .trim_matches(')')
        .to_string()
}

fn extract_moonbit_metadata(
    file_path: &str,
    source: &str,
    now: i64,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
    refs: &mut Vec<UnresolvedReference>,
) {
    let Ok(json) = serde_json::from_str::<serde_json::Value>(source) else {
        return;
    };
    if file_path.ends_with("moon.mod.json") {
        if let Some(name) = json.get("name").and_then(|v| v.as_str()) {
            let node = make_node(
                file_path,
                Language::MoonBit,
                NodeKind::Module,
                name,
                1,
                0,
                now,
                Some("moon.mod.json".into()),
            );
            add_contains(nodes, edges, &node);
            nodes.push(node);
        }
        return;
    }

    let package_name = json
        .get("name")
        .and_then(|v| v.as_str())
        .or_else(|| file_path.rsplit('/').nth(1))
        .unwrap_or("moonbit-package");
    let node = make_node(
        file_path,
        Language::MoonBit,
        NodeKind::Module,
        package_name,
        1,
        0,
        now,
        Some(file_path.rsplit('/').next().unwrap_or("moon.pkg").into()),
    );
    add_contains(nodes, edges, &node);
    let package_node_id = node.id.clone();
    nodes.push(node);

    if let Some(imports) = json.get("import").or_else(|| json.get("imports")) {
        if let Some(obj) = imports.as_object() {
            for (alias, value) in obj {
                let target = value.as_str().unwrap_or(alias);
                let import_node = make_node(
                    file_path,
                    Language::MoonBit,
                    NodeKind::Import,
                    alias,
                    1,
                    0,
                    now,
                    Some(target.to_string()),
                );
                add_contains(nodes, edges, &import_node);
                refs.push(unresolved(
                    &package_node_id,
                    alias,
                    EdgeKind::Imports,
                    file_path,
                    Language::MoonBit,
                    1,
                ));
                nodes.push(import_node);
            }
        }
    }
}

fn try_extract_rust_tree_sitter(
    file_path: &str,
    source: &str,
    now: i64,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
    refs: &mut Vec<UnresolvedReference>,
) -> bool {
    let mut parser = Parser::new();
    if parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .is_err()
    {
        return false;
    }
    let Some(tree) = parser.parse(source, None) else {
        return false;
    };
    if tree.root_node().has_error() {
        return false;
    }

    let root = tree.root_node();
    let mut stack = Vec::new();
    collect_rust_nodes(file_path, source, root, now, nodes, edges, refs, &mut stack);
    collect_rust_refs(file_path, source, root, nodes, refs);
    true
}

fn collect_rust_nodes(
    file_path: &str,
    source: &str,
    node: SyntaxNode,
    now: i64,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
    refs: &mut Vec<UnresolvedReference>,
    stack: &mut Vec<String>,
) {
    let kind = match node.kind() {
        "function_item" => {
            if rust_receiver_type(node, source).is_some() {
                Some(NodeKind::Method)
            } else {
                Some(NodeKind::Function)
            }
        }
        "struct_item" => Some(NodeKind::Struct),
        "trait_item" => Some(NodeKind::Trait),
        "enum_item" => Some(NodeKind::Enum),
        "enum_variant" => Some(NodeKind::EnumMember),
        "type_item" => Some(NodeKind::TypeAlias),
        "const_item" => Some(NodeKind::Constant),
        "static_item" => Some(NodeKind::Variable),
        "let_declaration" => Some(NodeKind::Variable),
        "field_declaration" => Some(NodeKind::Field),
        "function_signature_item" => Some(NodeKind::Method),
        "use_declaration" => Some(NodeKind::Import),
        "mod_item" => Some(NodeKind::Module),
        _ => None,
    };

    let mut pushed = false;
    if let Some(kind) = kind {
        if let Some(name) = rust_node_name(node, source, kind) {
            let signature = Some(
                node_text(node, source)
                    .lines()
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_string(),
            );
            let mut out =
                make_node_span(file_path, Language::Rust, kind, &name, node, now, signature);
            out.is_exported = rust_is_public(node, source);
            out.visibility = if out.is_exported {
                Some("public".into())
            } else if matches!(
                kind,
                NodeKind::Function
                    | NodeKind::Method
                    | NodeKind::Struct
                    | NodeKind::Trait
                    | NodeKind::Enum
                    | NodeKind::TypeAlias
            ) {
                Some("private".into())
            } else {
                None
            };
            out.is_async = node_text(node, source).trim_start().starts_with("async ")
                || node_text(node, source).contains(" async fn ");
            if kind == NodeKind::Method {
                if let Some(owner) = rust_receiver_type(node, source) {
                    out.qualified_name = format!("{owner}::{name}");
                }
            }
            add_contains_from_stack(nodes, edges, stack, &out, "tree-sitter");
            let id = out.id.clone();
            nodes.push(out);
            if matches!(
                kind,
                NodeKind::Struct
                    | NodeKind::Trait
                    | NodeKind::Enum
                    | NodeKind::Module
                    | NodeKind::Function
                    | NodeKind::Method
            ) {
                stack.push(id);
                pushed = true;
            }
        }
    }

    if node.kind() == "impl_item" {
        if let Some((trait_name, type_name)) = rust_impl_trait_for_type(node, source) {
            if let Some(type_node) = nodes.iter().find(|n| {
                n.name == type_name
                    && matches!(n.kind, NodeKind::Struct | NodeKind::Enum | NodeKind::Trait)
            }) {
                refs_push(
                    refs,
                    &type_node.id,
                    &trait_name,
                    EdgeKind::Implements,
                    file_path,
                    Language::Rust,
                    node.start_position().row as i64 + 1,
                    node.start_position().column as i64,
                );
            }
        }
    }

    for child in named_children(node) {
        collect_rust_nodes(file_path, source, child, now, nodes, edges, refs, stack);
    }

    if pushed {
        stack.pop();
    }
}

fn collect_rust_refs(
    file_path: &str,
    source: &str,
    node: SyntaxNode,
    nodes: &[Node],
    refs: &mut Vec<UnresolvedReference>,
) {
    match node.kind() {
        "use_declaration" => {
            if let Some(name) = rust_import_root(node, source) {
                refs_push(
                    refs,
                    &format!("file:{file_path}"),
                    &name,
                    EdgeKind::Imports,
                    file_path,
                    Language::Rust,
                    node.start_position().row as i64 + 1,
                    node.start_position().column as i64,
                );
            }
        }
        "call_expression" => {
            if let Some(function) = node.child_by_field_name("function") {
                if let Some(name) = callable_name(function, source) {
                    if let Some(caller) =
                        enclosing_callable(nodes, node.start_position().row as i64 + 1)
                    {
                        refs_push(
                            refs,
                            &caller.id,
                            &name,
                            EdgeKind::Calls,
                            file_path,
                            Language::Rust,
                            node.start_position().row as i64 + 1,
                            node.start_position().column as i64,
                        );
                    }
                }
            }
        }
        _ => {}
    }

    for child in named_children(node) {
        collect_rust_refs(file_path, source, child, nodes, refs);
    }
}

fn try_extract_moonbit_tree_sitter(
    file_path: &str,
    source: &str,
    now: i64,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
    refs: &mut Vec<UnresolvedReference>,
) -> bool {
    let mut parser = Parser::new();
    if parser
        .set_language(&tree_sitter_moonbit::LANGUAGE.into())
        .is_err()
    {
        return false;
    }
    let Some(tree) = parser.parse(source, None) else {
        return false;
    };
    if tree.root_node().has_error() {
        return false;
    }

    let root = tree.root_node();
    let mut stack = Vec::new();
    collect_moonbit_nodes(file_path, source, root, now, nodes, edges, &mut stack);
    collect_moonbit_refs(file_path, source, root, nodes, refs);
    true
}

fn collect_moonbit_nodes(
    file_path: &str,
    source: &str,
    node: SyntaxNode,
    now: i64,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
    stack: &mut Vec<String>,
) {
    let kind = match node.kind() {
        "function_definition" => Some(NodeKind::Function),
        "impl_definition" => Some(NodeKind::Method),
        "struct_definition" | "tuple_struct_definition" => Some(NodeKind::Struct),
        "trait_definition" => Some(NodeKind::Trait),
        "trait_method_declaration" => Some(NodeKind::Method),
        "enum_definition" => Some(NodeKind::Enum),
        "enum_constructor" => Some(NodeKind::EnumMember),
        "type_alias_definition" | "type_definition" => Some(NodeKind::TypeAlias),
        "const_definition" => Some(NodeKind::Constant),
        "import_declaration" => Some(NodeKind::Import),
        "package_declaration" => Some(NodeKind::Module),
        _ => None,
    };

    let mut pushed = false;
    if let Some(kind) = kind {
        if let Some(name) = moonbit_node_name(node, source, kind) {
            let signature = Some(
                node_text(node, source)
                    .lines()
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_string(),
            );
            let mut out = make_node_span(
                file_path,
                Language::MoonBit,
                kind,
                &name,
                node,
                now,
                signature,
            );
            out.is_exported = moonbit_is_public(node, source);
            out.visibility = if out.is_exported {
                Some("public".into())
            } else {
                None
            };
            if kind == NodeKind::Method {
                if let Some(owner) = moonbit_impl_owner(node, source) {
                    out.qualified_name = format!("{owner}::{name}");
                }
            }
            add_contains_from_stack(nodes, edges, stack, &out, "tree-sitter");
            let id = out.id.clone();
            nodes.push(out);
            if matches!(
                kind,
                NodeKind::Struct
                    | NodeKind::Trait
                    | NodeKind::Enum
                    | NodeKind::Module
                    | NodeKind::Function
                    | NodeKind::Method
            ) {
                stack.push(id);
                pushed = true;
            }
        }
    }

    for child in named_children(node) {
        collect_moonbit_nodes(file_path, source, child, now, nodes, edges, stack);
    }

    if pushed {
        stack.pop();
    }
}

fn collect_moonbit_refs(
    file_path: &str,
    source: &str,
    node: SyntaxNode,
    nodes: &[Node],
    refs: &mut Vec<UnresolvedReference>,
) {
    match node.kind() {
        "import_declaration" => {
            for child in named_children(node) {
                if child.kind() == "import_item" {
                    if let Some(name) = moonbit_import_name(child, source) {
                        refs_push(
                            refs,
                            &format!("file:{file_path}"),
                            &name,
                            EdgeKind::Imports,
                            file_path,
                            Language::MoonBit,
                            child.start_position().row as i64 + 1,
                            child.start_position().column as i64,
                        );
                    }
                }
            }
        }
        "apply_expression" | "dot_apply_expression" | "dot_dot_apply_expression" => {
            if let Some(name) = moonbit_call_name(node, source) {
                if let Some(caller) =
                    enclosing_callable(nodes, node.start_position().row as i64 + 1)
                {
                    refs_push(
                        refs,
                        &caller.id,
                        &name,
                        EdgeKind::Calls,
                        file_path,
                        Language::MoonBit,
                        node.start_position().row as i64 + 1,
                        node.start_position().column as i64,
                    );
                }
            }
        }
        _ => {}
    }

    for child in named_children(node) {
        collect_moonbit_refs(file_path, source, child, nodes, refs);
    }
}

fn extract_generic(
    file_path: &str,
    source: &str,
    language: Language,
    now: i64,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
    refs: &mut Vec<UnresolvedReference>,
) {
    add_regex_nodes(
        file_path,
        source,
        language,
        now,
        nodes,
        edges,
        r"(?m)^\s*(?:export\s+)?(?:async\s+)?function\s+([A-Za-z_$][A-Za-z0-9_$]*)",
        NodeKind::Function,
    );
    add_regex_nodes(
        file_path,
        source,
        language,
        now,
        nodes,
        edges,
        r"(?m)^\s*(?:export\s+)?class\s+([A-Za-z_$][A-Za-z0-9_$]*)",
        NodeKind::Class,
    );
    add_call_refs(
        file_path,
        source,
        language,
        nodes,
        refs,
        r"([A-Za-z_$][A-Za-z0-9_$.]*)\s*\(",
    );
}

fn add_regex_nodes(
    file_path: &str,
    source: &str,
    language: Language,
    now: i64,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
    pattern: &str,
    kind: NodeKind,
) {
    let re = Regex::new(pattern).unwrap();
    for cap in re.captures_iter(source) {
        let Some(name_match) = cap.get(2).or_else(|| cap.get(1)) else {
            continue;
        };
        let mut name = name_match.as_str().to_string();
        if kind == NodeKind::Method && name.contains("::") {
            name = name.rsplit("::").next().unwrap_or(&name).to_string();
        }
        let signature = cap.get(0).map(|m| m.as_str().trim().to_string());
        let line = line_for(source, name_match.start());
        let mut node = make_node(file_path, language, kind, &name, line, 0, now, signature);
        node.is_exported = cap
            .get(1)
            .map(|m| m.as_str().contains("pub") || m.as_str().contains("export"))
            .unwrap_or(false);
        node.visibility = if node.is_exported {
            Some("public".into())
        } else {
            None
        };
        add_contains(nodes, edges, &node);
        nodes.push(node);
    }
}

fn add_call_refs(
    file_path: &str,
    source: &str,
    language: Language,
    nodes: &[Node],
    refs: &mut Vec<UnresolvedReference>,
    pattern: &str,
) {
    let re = Regex::new(pattern).unwrap();
    let keywords = [
        "if", "for", "while", "match", "return", "fn", "test", "inspect", "Some", "Ok", "Err",
    ];
    for cap in re.captures_iter(source) {
        let name = cap.get(1).unwrap().as_str().rsplit("::").next().unwrap();
        if keywords.contains(&name) {
            continue;
        }
        let line = line_for(source, cap.get(1).unwrap().start());
        if let Some(caller) = nodes
            .iter()
            .filter(|n| matches!(n.kind, NodeKind::Function | NodeKind::Method))
            .rev()
            .find(|n| n.start_line <= line)
        {
            refs.push(unresolved(
                &caller.id,
                name,
                EdgeKind::Calls,
                file_path,
                language,
                line,
            ));
        }
    }
}

fn make_node(
    file_path: &str,
    language: Language,
    kind: NodeKind,
    name: &str,
    line: i64,
    col: i64,
    now: i64,
    signature: Option<String>,
) -> Node {
    Node {
        id: format!("{}:{}:{}:{}", kind.as_str(), file_path, name, line),
        kind,
        name: name.to_string(),
        qualified_name: name.to_string(),
        file_path: file_path.to_string(),
        language,
        start_line: line,
        end_line: line,
        start_column: col,
        end_column: col,
        docstring: None,
        signature,
        visibility: None,
        is_exported: false,
        is_async: false,
        is_static: false,
        is_abstract: false,
        updated_at: now,
    }
}

fn make_node_span(
    file_path: &str,
    language: Language,
    kind: NodeKind,
    name: &str,
    node: SyntaxNode,
    now: i64,
    signature: Option<String>,
) -> Node {
    let start = node.start_position();
    let end = node.end_position();
    Node {
        id: format!("{}:{}:{}:{}", kind.as_str(), file_path, name, start.row + 1),
        kind,
        name: name.to_string(),
        qualified_name: name.to_string(),
        file_path: file_path.to_string(),
        language,
        start_line: start.row as i64 + 1,
        end_line: end.row as i64 + 1,
        start_column: start.column as i64,
        end_column: end.column as i64,
        docstring: None,
        signature,
        visibility: None,
        is_exported: false,
        is_async: false,
        is_static: false,
        is_abstract: false,
        updated_at: now,
    }
}

fn add_contains(nodes: &[Node], edges: &mut Vec<Edge>, node: &Node) {
    if let Some(file) = nodes.first() {
        edges.push(Edge {
            id: None,
            source: file.id.clone(),
            target: node.id.clone(),
            kind: EdgeKind::Contains,
            line: None,
            col: None,
            provenance: Some("regex".into()),
        });
    }
}

fn add_contains_from_stack(
    nodes: &[Node],
    edges: &mut Vec<Edge>,
    stack: &[String],
    node: &Node,
    provenance: &str,
) {
    let source = stack
        .last()
        .cloned()
        .or_else(|| nodes.first().map(|n| n.id.clone()));
    if let Some(source) = source {
        edges.push(Edge {
            id: None,
            source,
            target: node.id.clone(),
            kind: EdgeKind::Contains,
            line: None,
            col: None,
            provenance: Some(provenance.into()),
        });
    }
}

fn unresolved(
    from: &str,
    name: &str,
    kind: EdgeKind,
    file_path: &str,
    language: Language,
    line: i64,
) -> UnresolvedReference {
    UnresolvedReference {
        from_node_id: from.to_string(),
        reference_name: name.to_string(),
        reference_kind: kind,
        line,
        column: 0,
        file_path: file_path.to_string(),
        language,
    }
}

fn refs_push(
    refs: &mut Vec<UnresolvedReference>,
    from: &str,
    name: &str,
    kind: EdgeKind,
    file_path: &str,
    language: Language,
    line: i64,
    column: i64,
) {
    if !name.is_empty() {
        refs.push(UnresolvedReference {
            from_node_id: from.to_string(),
            reference_name: name.to_string(),
            reference_kind: kind,
            line,
            column,
            file_path: file_path.to_string(),
            language,
        });
    }
}

fn named_children(node: SyntaxNode) -> Vec<SyntaxNode> {
    (0..node.named_child_count())
        .filter_map(|i| node.named_child(i as u32))
        .collect()
}

fn node_text<'a>(node: SyntaxNode, source: &'a str) -> &'a str {
    source.get(node.byte_range()).unwrap_or_default()
}

fn child_text_by_kind<'a>(node: SyntaxNode, source: &'a str, kinds: &[&str]) -> Option<&'a str> {
    named_children(node)
        .into_iter()
        .find(|child| kinds.contains(&child.kind()))
        .map(|child| node_text(child, source))
}

fn descendant_text_by_kind<'a>(
    node: SyntaxNode,
    source: &'a str,
    kinds: &[&str],
) -> Option<&'a str> {
    if kinds.contains(&node.kind()) {
        return Some(node_text(node, source));
    }
    for child in named_children(node) {
        if let Some(text) = descendant_text_by_kind(child, source, kinds) {
            return Some(text);
        }
    }
    None
}

fn rust_node_name(node: SyntaxNode, source: &str, kind: NodeKind) -> Option<String> {
    if kind == NodeKind::Import {
        return rust_import_root(node, source);
    }
    if kind == NodeKind::Variable && node.kind() == "let_declaration" {
        return descendant_text_by_kind(node, source, &["identifier"]).map(clean_symbol_name);
    }
    if kind == NodeKind::Field {
        return child_text_by_kind(node, source, &["field_identifier", "identifier"])
            .map(clean_symbol_name);
    }
    node.child_by_field_name("name")
        .map(|n| clean_symbol_name(node_text(n, source)))
        .or_else(|| {
            child_text_by_kind(
                node,
                source,
                &["identifier", "type_identifier", "field_identifier"],
            )
            .map(clean_symbol_name)
        })
}

fn rust_is_public(node: SyntaxNode, source: &str) -> bool {
    node_text(node, source).trim_start().starts_with("pub")
        || named_children(node).into_iter().any(|child| {
            child.kind() == "visibility_modifier" && node_text(child, source).contains("pub")
        })
}

fn rust_receiver_type(node: SyntaxNode, source: &str) -> Option<String> {
    let mut parent = node.parent();
    while let Some(p) = parent {
        if p.kind() == "impl_item" {
            let mut direct = named_children(p)
                .into_iter()
                .filter(|child| {
                    matches!(
                        child.kind(),
                        "type_identifier" | "generic_type" | "scoped_type_identifier"
                    )
                })
                .collect::<Vec<_>>();
            if let Some(last) = direct.pop() {
                return Some(clean_type_name(node_text(last, source)));
            }
            return descendant_text_by_kind(p, source, &["type_identifier"]).map(clean_type_name);
        }
        parent = p.parent();
    }
    None
}

fn rust_impl_trait_for_type(node: SyntaxNode, source: &str) -> Option<(String, String)> {
    if node.kind() != "impl_item" || !node_text(node, source).contains(" for ") {
        return None;
    }
    let names: Vec<String> = named_children(node)
        .into_iter()
        .filter(|child| {
            matches!(
                child.kind(),
                "type_identifier" | "generic_type" | "scoped_type_identifier"
            )
        })
        .map(|child| clean_type_name(node_text(child, source)))
        .collect();
    if names.len() >= 2 {
        Some((names[0].clone(), names[names.len() - 1].clone()))
    } else {
        None
    }
}

fn rust_import_root(node: SyntaxNode, source: &str) -> Option<String> {
    let text = node_text(node, source)
        .trim()
        .strip_prefix("use")
        .unwrap_or(node_text(node, source))
        .trim()
        .trim_end_matches(';')
        .trim();
    text.split("::")
        .next()
        .map(|s| s.trim_matches('{').trim().to_string())
        .filter(|s| !s.is_empty())
}

fn callable_name(node: SyntaxNode, source: &str) -> Option<String> {
    match node.kind() {
        "identifier" | "field_identifier" => Some(clean_symbol_name(node_text(node, source))),
        "scoped_identifier" => node_text(node, source)
            .rsplit("::")
            .next()
            .map(clean_symbol_name),
        "field_expression" => node
            .child_by_field_name("field")
            .map(|field| clean_symbol_name(node_text(field, source))),
        "generic_function" => named_children(node)
            .into_iter()
            .find_map(|child| callable_name(child, source)),
        _ => None,
    }
}

fn moonbit_node_name(node: SyntaxNode, source: &str, kind: NodeKind) -> Option<String> {
    match kind {
        NodeKind::Function | NodeKind::Method => child_text_by_kind(
            node,
            source,
            &["function_identifier", "lowercase_identifier", "identifier"],
        )
        .map(|s| clean_symbol_name(s.rsplit("::").next().unwrap_or(s))),
        NodeKind::Struct | NodeKind::Trait | NodeKind::Enum => child_text_by_kind(
            node,
            source,
            &[
                "identifier",
                "type_identifier",
                "type_name",
                "uppercase_identifier",
            ],
        )
        .map(clean_symbol_name),
        NodeKind::EnumMember => child_text_by_kind(
            node,
            source,
            &["uppercase_identifier", "identifier", "type_name"],
        )
        .map(clean_symbol_name),
        NodeKind::TypeAlias => descendant_text_by_kind(
            node,
            source,
            &[
                "type_identifier",
                "type_name",
                "identifier",
                "uppercase_identifier",
            ],
        )
        .map(clean_symbol_name),
        NodeKind::Constant => {
            child_text_by_kind(node, source, &["uppercase_identifier", "identifier"])
                .map(clean_symbol_name)
        }
        NodeKind::Import => moonbit_import_name(node, source),
        NodeKind::Module => node
            .named_child(0)
            .map(|child| clean_quoted(node_text(child, source))),
        _ => None,
    }
}

fn moonbit_is_public(node: SyntaxNode, source: &str) -> bool {
    named_children(node)
        .into_iter()
        .any(|child| child.kind() == "visibility" && node_text(child, source).contains("pub"))
        || node_text(node, source).trim_start().starts_with("pub ")
}

fn moonbit_impl_owner(node: SyntaxNode, source: &str) -> Option<String> {
    child_text_by_kind(
        node,
        source,
        &["type_name", "type_identifier", "qualified_type_identifier"],
    )
    .map(clean_type_name)
}

fn moonbit_import_name(node: SyntaxNode, source: &str) -> Option<String> {
    if node.kind() == "import_declaration" {
        return named_children(node)
            .into_iter()
            .find(|child| child.kind() == "import_item")
            .and_then(|child| moonbit_import_name(child, source));
    }
    named_children(node)
        .into_iter()
        .find(|child| child.kind() == "string_literal")
        .map(|child| clean_quoted(node_text(child, source)))
}

fn moonbit_call_name(node: SyntaxNode, source: &str) -> Option<String> {
    for child in named_children(node) {
        match child.kind() {
            "qualified_identifier" | "function_identifier" | "method_expression" => {
                let text = node_text(child, source);
                let name = text
                    .rsplit(['.', ':'])
                    .find(|part| !part.is_empty())
                    .unwrap_or(text);
                return Some(clean_symbol_name(name));
            }
            "lowercase_identifier" | "identifier" => {
                return Some(clean_symbol_name(node_text(child, source)));
            }
            _ => {}
        }
    }
    None
}

fn enclosing_callable(nodes: &[Node], line: i64) -> Option<&Node> {
    nodes
        .iter()
        .filter(|n| matches!(n.kind, NodeKind::Function | NodeKind::Method))
        .filter(|n| n.start_line <= line && line <= n.end_line.max(n.start_line))
        .min_by_key(|n| n.end_line - n.start_line)
}

fn clean_symbol_name(s: &str) -> String {
    s.trim()
        .trim_matches('"')
        .trim_matches('\'')
        .trim_start_matches('.')
        .to_string()
}

fn clean_quoted(s: &str) -> String {
    s.trim().trim_matches('"').trim_matches('\'').to_string()
}

fn clean_type_name(s: &str) -> String {
    let s = s.trim();
    let before_generics = s.split('<').next().unwrap_or(s);
    before_generics
        .rsplit("::")
        .next()
        .unwrap_or(before_generics)
        .trim()
        .to_string()
}

fn line_for(source: &str, idx: usize) -> i64 {
    source[..idx.min(source.len())]
        .bytes()
        .filter(|b| *b == b'\n')
        .count() as i64
        + 1
}

fn extract_mbt_markdown_code_with_padding(source: &str) -> String {
    let mut out = String::new();
    let mut in_mbt = false;
    for line in source.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            in_mbt = trimmed.contains("mbt");
            out.push('\n');
            continue;
        }
        if in_mbt {
            out.push_str(line);
        }
        out.push('\n');
    }
    out
}

fn strip_moonbit_comments_preserve_lines(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    let mut in_string = false;
    let mut escaped = false;
    while let Some(ch) = chars.next() {
        if in_string {
            out.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
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
            chars.next();
            out.push(' ');
            out.push(' ');
            for next in chars.by_ref() {
                if next == '\n' {
                    out.push('\n');
                    break;
                }
                out.push(' ');
            }
            continue;
        }

        if ch == '/' && chars.peek() == Some(&'*') {
            chars.next();
            out.push(' ');
            out.push(' ');
            let mut prev = '\0';
            for next in chars.by_ref() {
                if next == '\n' {
                    out.push('\n');
                } else {
                    out.push(' ');
                }
                if prev == '*' && next == '/' {
                    break;
                }
                prev = next;
            }
            continue;
        }

        out.push(ch);
    }
    out
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or_default()
}
