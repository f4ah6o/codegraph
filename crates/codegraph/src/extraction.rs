use crate::config::CodeGraphConfig;
use crate::types::*;
use regex::Regex;
use std::path::Path;
use std::sync::LazyLock;
use tree_sitter::{Node as SyntaxNode, Parser};

static SOL_ROUTE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"@sol\.(route|api_get|api_post|api_put|api_patch|api_delete)\s*\(\s*"([^"]+)"\s*,\s*([A-Za-z_][A-Za-z0-9_]*)"#,
    )
    .unwrap()
});

static JS_ROUTE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"\b(?:app|router|route)\.(get|post|put|patch|delete|all|use)\s*\(\s*["']([^"']+)["']\s*,\s*([^\n;]+)"#,
    )
    .unwrap()
});

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

    match language {
        Language::Rust => extract_rust(&file_path, source, now, &mut nodes, &mut edges, &mut refs),
        Language::MoonBit => {
            extract_moonbit(&file_path, source, now, &mut nodes, &mut edges, &mut refs)
        }
        _ => extract_generic(
            &file_path, source, language, now, &mut nodes, &mut edges, &mut refs,
        ),
    }
    extract_framework_routes(
        &file_path, source, language, now, &mut nodes, &mut edges, &mut refs,
    );

    ExtractionResult {
        nodes,
        edges,
        unresolved_references: refs,
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

fn extract_framework_routes(
    file_path: &str,
    source: &str,
    language: Language,
    now: i64,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
    refs: &mut Vec<UnresolvedReference>,
) {
    match language {
        Language::MoonBit => extract_sol_routes(file_path, source, now, nodes, edges, refs),
        Language::JavaScript | Language::TypeScript | Language::Tsx | Language::Jsx => {
            extract_js_routes(file_path, source, language, now, nodes, edges, refs);
            extract_file_routes(file_path, source, language, now, nodes, edges);
        }
        Language::Svelte => extract_file_routes(file_path, source, language, now, nodes, edges),
        _ => {}
    }
}

fn extract_sol_routes(
    file_path: &str,
    source: &str,
    now: i64,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
    refs: &mut Vec<UnresolvedReference>,
) {
    let safe = strip_line_comments(source);
    for cap in SOL_ROUTE_RE.captures_iter(&safe) {
        let kind = cap.get(1).unwrap().as_str();
        let route_path = cap.get(2).unwrap().as_str();
        let handler = cap.get(3).unwrap().as_str();
        let method = match kind {
            "api_get" => Some("GET"),
            "api_post" => Some("POST"),
            "api_put" => Some("PUT"),
            "api_patch" => Some("PATCH"),
            "api_delete" => Some("DELETE"),
            _ => None,
        };
        add_route_node(
            file_path,
            Language::MoonBit,
            method,
            route_path,
            Some(handler),
            line_for(&safe, cap.get(0).unwrap().start()),
            cap.get(0).unwrap().as_str(),
            now,
            nodes,
            edges,
            refs,
        );
    }
}

fn extract_js_routes(
    file_path: &str,
    source: &str,
    language: Language,
    now: i64,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
    refs: &mut Vec<UnresolvedReference>,
) {
    let safe = strip_js_comments(source);
    for cap in JS_ROUTE_RE.captures_iter(&safe) {
        let method = cap.get(1).unwrap().as_str().to_ascii_uppercase();
        let route_path = cap.get(2).unwrap().as_str();
        if method == "USE" && !route_path.starts_with('/') {
            continue;
        }
        let handlers = cap.get(3).unwrap().as_str();
        let handler = tail_handler_ident(handlers);
        add_route_node(
            file_path,
            language,
            Some(&method),
            route_path,
            handler.as_deref(),
            line_for(&safe, cap.get(0).unwrap().start()),
            cap.get(0).unwrap().as_str(),
            now,
            nodes,
            edges,
            refs,
        );
    }
}

fn extract_file_routes(
    file_path: &str,
    source: &str,
    language: Language,
    now: i64,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
) {
    if let Some((method, route_path)) = file_route_info(file_path) {
        let line = source
            .find("export default")
            .or_else(|| source.find("export "))
            .map(|idx| line_for(source, idx))
            .unwrap_or(1);
        add_route_node(
            file_path,
            language,
            method,
            &route_path,
            None,
            line,
            file_path,
            now,
            nodes,
            edges,
            &mut Vec::new(),
        );
    }
}

fn add_route_node(
    file_path: &str,
    language: Language,
    method: Option<&str>,
    route_path: &str,
    handler: Option<&str>,
    line: i64,
    signature: &str,
    now: i64,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
    refs: &mut Vec<UnresolvedReference>,
) {
    let name = method
        .map(|m| format!("{m} {route_path}"))
        .unwrap_or_else(|| route_path.to_string());
    if nodes.iter().any(|n| {
        n.kind == NodeKind::Route
            && n.file_path == file_path
            && n.name == name
            && n.start_line == line
    }) {
        return;
    }
    let mut node = make_node(
        file_path,
        language,
        NodeKind::Route,
        &name,
        line,
        0,
        now,
        Some(signature.trim().to_string()),
    );
    node.qualified_name = format!("{file_path}::route:{name}");
    add_contains(nodes, edges, &node);
    let id = node.id.clone();
    nodes.push(node);
    if let Some(handler) = handler.filter(|h| !h.is_empty()) {
        refs.push(unresolved(
            &id,
            handler,
            EdgeKind::References,
            file_path,
            language,
            line,
        ));
    }
}

fn tail_handler_ident(handlers: &str) -> Option<String> {
    let last = handlers
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .last()?;
    if last.contains("=>") || last.starts_with("function") || last.starts_with("async ") {
        return None;
    }
    last.trim_end_matches(')')
        .split('.')
        .next_back()
        .and_then(|s| {
            s.trim()
                .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                .next()
        })
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn file_route_info(file_path: &str) -> Option<(Option<&'static str>, String)> {
    let normalized = file_path.replace('\\', "/");
    if normalized.starts_with("src/routes/") {
        return sveltekit_route(&format!("/{normalized}"));
    }
    if normalized.starts_with("routes/") {
        return sveltekit_route(&format!("/{normalized}"));
    }
    if normalized.starts_with("pages/") || normalized.starts_with("src/pages/") {
        return next_pages_route(&normalized);
    }
    if normalized.starts_with("app/") || normalized.starts_with("src/app/") {
        return next_app_route(&normalized);
    }
    None
}

fn sveltekit_route(path: &str) -> Option<(Option<&'static str>, String)> {
    let idx = path.find("/routes/")? + "/routes/".len();
    let rel = &path[idx..];
    if !rel.rsplit('/').next()?.starts_with('+') {
        return None;
    }
    let dir = rel.rsplit_once('/').map(|(dir, _)| dir).unwrap_or("");
    Some((None, route_from_segments(dir)))
}

fn next_pages_route(path: &str) -> Option<(Option<&'static str>, String)> {
    if path.starts_with("src/pages/") {
        let rel = &path["src/".len()..];
        return next_pages_route(rel);
    }
    let idx = path.find("pages/")? + "pages/".len();
    let mut rel = strip_known_extension(&path[idx..])?.to_string();
    let basename = rel.rsplit('/').next().unwrap_or("");
    if basename.starts_with('_') {
        return None;
    }
    if rel == "_app" || rel == "_document" || rel == "_error" || rel.ends_with("/_middleware") {
        return None;
    }
    if rel.starts_with("api/") {
        rel = rel.trim_start_matches("api/").to_string();
        let child = route_from_segments(&rel);
        let route = if child == "/" {
            "/api".to_string()
        } else {
            format!("/api{child}")
        };
        return Some((Some("API"), route));
    }
    Some((None, route_from_segments(&rel)))
}

fn next_app_route(path: &str) -> Option<(Option<&'static str>, String)> {
    if path.starts_with("src/app/") {
        let rel = &path["src/".len()..];
        return next_app_route(rel);
    }
    let idx = path.find("app/")? + "app/".len();
    let rel = &path[idx..];
    let file = rel.rsplit('/').next()?;
    if !matches!(
        file,
        "page.tsx" | "page.jsx" | "page.ts" | "page.js" | "route.ts" | "route.js"
    ) {
        return None;
    }
    let dir = rel.rsplit_once('/').map(|(dir, _)| dir).unwrap_or("");
    let method = if file.starts_with("route.") {
        Some("API")
    } else {
        None
    };
    Some((method, route_from_segments(dir)))
}

fn strip_known_extension(path: &str) -> Option<&str> {
    [".tsx", ".ts", ".jsx", ".js", ".svelte"]
        .iter()
        .find_map(|ext| path.strip_suffix(ext))
}

fn route_from_segments(path: &str) -> String {
    let mut segments = Vec::new();
    for segment in path.split('/').filter(|s| !s.is_empty()) {
        if segment == "index" || segment.starts_with('(') && segment.ends_with(')') {
            continue;
        }
        if segment.starts_with('[') && segment.ends_with(']') {
            let param = segment
                .trim_start_matches('[')
                .trim_end_matches(']')
                .trim_start_matches("...");
            segments.push(format!(":{param}"));
        } else {
            segments.push(segment.to_string());
        }
    }
    if segments.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", segments.join("/"))
    }
}

fn strip_line_comments(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    let mut in_string: Option<char> = None;
    let mut escaped = false;
    while let Some(ch) = chars.next() {
        if let Some(quote) = in_string {
            out.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == quote {
                in_string = None;
            }
            continue;
        }
        if ch == '"' || ch == '\'' || ch == '`' {
            in_string = Some(ch);
            out.push(ch);
            continue;
        }
        if ch == '/' && chars.peek() == Some(&'/') {
            chars.next();
            out.push(' ');
            out.push(' ');
            while let Some(next) = chars.next() {
                if next == '\n' {
                    out.push('\n');
                    break;
                }
                out.push(' ');
            }
            continue;
        }
        out.push(ch);
    }
    out
}

fn strip_js_comments(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    let mut in_string: Option<char> = None;
    let mut escaped = false;
    while let Some(ch) = chars.next() {
        if let Some(quote) = in_string {
            out.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == quote {
                in_string = None;
            }
            continue;
        }
        if ch == '"' || ch == '\'' || ch == '`' {
            in_string = Some(ch);
            out.push(ch);
            continue;
        }
        if ch == '/' && chars.peek() == Some(&'/') {
            chars.next();
            out.push(' ');
            out.push(' ');
            while let Some(next) = chars.next() {
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

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or_default()
}
