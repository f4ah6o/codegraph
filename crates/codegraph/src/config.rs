use crate::types::Language;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeGraphConfig {
    pub version: i64,
    #[serde(rename = "rootDir")]
    pub root_dir: String,
    pub include: Vec<String>,
    pub exclude: Vec<String>,
    pub languages: Vec<Language>,
    pub frameworks: Vec<serde_json::Value>,
    #[serde(rename = "maxFileSize")]
    pub max_file_size: u64,
    #[serde(rename = "extractDocstrings")]
    pub extract_docstrings: bool,
    #[serde(rename = "trackCallSites")]
    pub track_call_sites: bool,
}

impl CodeGraphConfig {
    pub fn default_for_root(root: impl Into<String>) -> Self {
        Self {
            version: 1,
            root_dir: root.into(),
            include: vec![
                "**/*.ts",
                "**/*.tsx",
                "**/*.js",
                "**/*.jsx",
                "**/*.py",
                "**/*.go",
                "**/*.rs",
                "**/*.java",
                "**/*.c",
                "**/*.h",
                "**/*.cpp",
                "**/*.hpp",
                "**/*.cc",
                "**/*.cxx",
                "**/*.cs",
                "**/*.php",
                "**/*.rb",
                "**/*.swift",
                "**/*.kt",
                "**/*.kts",
                "**/*.dart",
                "**/*.svelte",
                "**/*.vue",
                "**/*.liquid",
                "**/*.pas",
                "**/*.dpr",
                "**/*.dpk",
                "**/*.lpr",
                "**/*.dfm",
                "**/*.fmx",
                "**/*.scala",
                "**/*.sc",
                "**/*.mbt",
                "**/*.mbti",
                "**/*.mbt.md",
                "**/moon.mod.json",
                "**/moon.pkg.json",
                "**/moon.pkg",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            exclude: vec![
                "**/.git/**",
                "**/node_modules/**",
                "**/vendor/**",
                "**/dist/**",
                "**/build/**",
                "**/out/**",
                "**/target/**",
                "**/.codegraph/**",
                "**/.moon/**",
                "**/.mooncakes/**",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            languages: Vec::new(),
            frameworks: Vec::new(),
            max_file_size: 1_048_576,
            extract_docstrings: true,
            track_call_sites: true,
        }
    }
}

pub fn config_path(root: &Path) -> std::path::PathBuf {
    root.join(".codegraph").join("config.json")
}

pub fn load_config(root: &Path) -> Result<CodeGraphConfig> {
    let path = config_path(root);
    let text = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let mut cfg: CodeGraphConfig =
        serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
    if cfg.include.is_empty() {
        cfg.include = CodeGraphConfig::default_for_root(".").include;
    }
    Ok(cfg)
}

pub fn save_config(root: &Path, config: &CodeGraphConfig) -> Result<()> {
    let path = config_path(root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let text = serde_json::to_string_pretty(config)? + "\n";
    fs::write(&path, text).with_context(|| format!("writing {}", path.display()))
}
