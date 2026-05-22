use codegraph::extraction::{detect_language, extract_from_source};
use codegraph::types::{EdgeKind, ExtractionResult, Language, NodeKind};
use codegraph::CodeGraph;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

pub struct OriginalFixtureProject {
    _dir: TempDir,
    root: PathBuf,
}

impl OriginalFixtureProject {
    pub fn new(files: &[(&str, &str)]) -> Self {
        let dir = TempDir::new().unwrap();
        for (path, source) in files {
            let full_path = dir.path().join(path);
            if let Some(parent) = full_path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(full_path, source).unwrap();
        }
        Self {
            root: dir.path().to_path_buf(),
            _dir: dir,
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn index(&self) -> CodeGraph {
        let mut cg = CodeGraph::init(&self.root).unwrap();
        let result = cg.index_all().unwrap();
        assert!(result.success, "{:?}", result.errors);
        cg
    }
}

pub struct OriginalSourceFixture {
    path: PathBuf,
    source: String,
    language: Language,
    result: ExtractionResult,
}

impl OriginalSourceFixture {
    pub fn new(path: &str, source: &str) -> Self {
        let path = PathBuf::from(path);
        let language = detect_language(&path, source);
        assert_ne!(language, Language::Unknown, "unsupported fixture: {path:?}");
        let result = extract_from_source(&path, source, language);
        Self {
            path,
            source: source.to_string(),
            language,
            result,
        }
    }

    pub fn language(&self) -> Language {
        self.language
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn result(&self) -> &ExtractionResult {
        &self.result
    }

    pub fn assert_node(&self, kind: NodeKind, name: &str) {
        assert!(
            self.result
                .nodes
                .iter()
                .any(|node| node.kind == kind && node.name == name),
            "missing {kind:?} node {name:?} in {}: {:?}",
            self.path.display(),
            self.result
                .nodes
                .iter()
                .map(|node| (node.kind, node.name.as_str()))
                .collect::<Vec<_>>()
        );
    }

    pub fn assert_reference(&self, kind: EdgeKind, name: &str) {
        assert!(
            self.result
                .unresolved_references
                .iter()
                .any(|reference| reference.reference_kind == kind
                    && reference.reference_name == name),
            "missing {kind:?} reference {name:?} in {}: {:?}",
            self.path.display(),
            self.result
                .unresolved_references
                .iter()
                .map(|reference| (reference.reference_kind, reference.reference_name.as_str()))
                .collect::<Vec<_>>()
        );
    }
}
