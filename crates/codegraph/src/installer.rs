use anyhow::{anyhow, Context, Result};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

const CODEGRAPH_SERVER: &str = "codegraph";
const SECTION_START: &str = "<!-- CODEGRAPH_START -->";
const SECTION_END: &str = "<!-- CODEGRAPH_END -->";
const CLAUDE_MD_SECTION: &str = r#"<!-- CODEGRAPH_START -->
## CodeGraph

CodeGraph builds a local semantic graph for source exploration.

- Start with `cgz status` before relying on indexed results.
- Use `cgz query <term>` to find symbols by name.
- Use `cgz context <task>` for task-oriented evidence.
- Use `cgz affected <files>` before changing files with likely tests.
- Treat CodeGraph output as navigation evidence; final validation still comes from the project's tests, type checks, or build checks.
<!-- CODEGRAPH_END -->"#;

const CODEGRAPH_PERMISSIONS: &[&str] = &[
    "mcp__codegraph__codegraph_status",
    "mcp__codegraph__codegraph_files",
    "mcp__codegraph__codegraph_search",
    "mcp__codegraph__codegraph_context",
    "mcp__codegraph__codegraph_callers",
    "mcp__codegraph__codegraph_callees",
    "mcp__codegraph__codegraph_impact",
    "mcp__codegraph__codegraph_node",
    "mcp__codegraph__codegraph_explore",
];

#[derive(Debug, Clone, Default)]
pub struct InstallOptions {
    pub global: bool,
    pub local: bool,
    pub yes: bool,
    pub no_init: bool,
    pub allow_permissions: bool,
    pub project_path: Option<PathBuf>,
    pub home_dir: Option<PathBuf>,
}

#[derive(Debug)]
pub struct InstallResult {
    pub claude_json_path: PathBuf,
    pub claude_json_changed: bool,
    pub settings_json_path: Option<PathBuf>,
    pub settings_json_changed: bool,
    pub claude_md_path: PathBuf,
    pub claude_md_changed: bool,
    pub initialized: bool,
    pub init_message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallTarget {
    Global,
    Local,
}

pub fn install(options: &InstallOptions) -> Result<InstallResult> {
    let target = match (options.global, options.local) {
        (true, true) => return Err(anyhow!("install target must be either --global or --local")),
        (true, false) => InstallTarget::Global,
        (false, true) => InstallTarget::Local,
        (false, false) => return Err(anyhow!("install target must be --global or --local")),
    };
    let project_path = options
        .project_path
        .clone()
        .unwrap_or(std::env::current_dir()?)
        .canonicalize()
        .unwrap_or_else(|_| {
            options
                .project_path
                .clone()
                .unwrap_or_else(|| PathBuf::from("."))
        });
    let paths = install_paths(target, &project_path, options.home_dir.as_deref())?;

    let claude_json_changed = write_mcp_config(&paths.claude_json)?;
    let settings_json_changed = if options.allow_permissions {
        write_permissions(&paths.settings_json)?
    } else {
        false
    };
    let claude_md_changed = write_claude_md(&paths.claude_md)?;

    let mut initialized = false;
    let mut init_message = String::new();
    if target == InstallTarget::Local && !options.no_init {
        if crate::is_initialized(&project_path) {
            init_message = format!(
                "CodeGraph already initialized in {}",
                project_path.display()
            );
        } else if options.yes {
            let mut cg = crate::CodeGraph::init(&project_path)?;
            let result = cg.index_all()?;
            initialized = true;
            init_message = format!(
                "Initialized and indexed {} files ({} nodes, {} edges)",
                result.files_indexed, result.nodes_created, result.edges_created
            );
        } else {
            init_message =
                "Skipped project initialization. Re-run with --yes or run `cgz init -i` manually."
                    .to_string();
        }
    }

    Ok(InstallResult {
        claude_json_path: paths.claude_json,
        claude_json_changed,
        settings_json_path: options.allow_permissions.then_some(paths.settings_json),
        settings_json_changed,
        claude_md_path: paths.claude_md,
        claude_md_changed,
        initialized,
        init_message,
    })
}

struct InstallPaths {
    claude_json: PathBuf,
    settings_json: PathBuf,
    claude_md: PathBuf,
}

fn install_paths(
    target: InstallTarget,
    project_path: &Path,
    home_dir: Option<&Path>,
) -> Result<InstallPaths> {
    match target {
        InstallTarget::Global => {
            let home = home_dir
                .map(Path::to_path_buf)
                .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
                .ok_or_else(|| anyhow!("Could not determine HOME directory"))?;
            Ok(InstallPaths {
                claude_json: home.join(".claude.json"),
                settings_json: home.join(".claude").join("settings.json"),
                claude_md: home.join(".claude").join("CLAUDE.md"),
            })
        }
        InstallTarget::Local => Ok(InstallPaths {
            claude_json: project_path.join(".claude.json"),
            settings_json: project_path.join(".claude").join("settings.json"),
            claude_md: project_path.join(".claude").join("CLAUDE.md"),
        }),
    }
}

fn write_mcp_config(path: &Path) -> Result<bool> {
    let mut config = read_json_object(path)?;
    let mcp_servers = object_entry(&mut config, "mcpServers", path)?;
    let server_config = json!({
        "type": "stdio",
        "command": "cgz",
        "args": ["serve", "--mcp"],
    });
    let changed = mcp_servers.get(CODEGRAPH_SERVER) != Some(&server_config);
    mcp_servers.insert(CODEGRAPH_SERVER.to_string(), server_config);
    write_json_if_changed(path, &config, changed)?;
    Ok(changed)
}

fn write_permissions(path: &Path) -> Result<bool> {
    let mut settings = read_json_object(path)?;
    let permissions = object_entry(&mut settings, "permissions", path)?;
    let allow = permissions
        .entry("allow".to_string())
        .or_insert_with(|| json!([]));
    let allow = allow
        .as_array_mut()
        .ok_or_else(|| anyhow!("permissions.allow in {} is not an array", path.display()))?;

    let mut changed = false;
    for permission in CODEGRAPH_PERMISSIONS {
        if !allow.iter().any(|entry| entry.as_str() == Some(permission)) {
            allow.push(json!(permission));
            changed = true;
        }
    }
    write_json_if_changed(path, &settings, changed)?;
    Ok(changed)
}

fn write_claude_md(path: &Path) -> Result<bool> {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(err) => return Err(err).with_context(|| format!("reading {}", path.display())),
    };
    let updated = upsert_claude_section(&content);
    if updated == content {
        return Ok(false);
    }
    atomic_write(path, updated.as_bytes())?;
    Ok(true)
}

fn read_json_object(path: &Path) -> Result<Value> {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(json!({})),
        Err(err) => return Err(err).with_context(|| format!("reading {}", path.display())),
    };
    let parsed: Value =
        serde_json::from_str(&content).with_context(|| format!("parsing {}", path.display()))?;
    if parsed.is_object() {
        Ok(parsed)
    } else {
        Err(anyhow!("{} must contain a JSON object", path.display()))
    }
}

fn object_entry<'a>(
    value: &'a mut Value,
    key: &str,
    path: &Path,
) -> Result<&'a mut serde_json::Map<String, Value>> {
    value
        .as_object_mut()
        .expect("read_json_object returns a JSON object")
        .entry(key.to_string())
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or_else(|| anyhow!("{key} in {} is not a JSON object", path.display()))
}

fn write_json_if_changed(path: &Path, value: &Value, changed: bool) -> Result<()> {
    if changed || !path.exists() {
        let output = serde_json::to_string_pretty(value)? + "\n";
        atomic_write(path, output.as_bytes())?;
    }
    Ok(())
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating directory {}", parent.display()))?;
    }
    let tmp = path.with_extension(format!(
        "{}tmp",
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| format!("{ext}."))
            .unwrap_or_default()
    ));
    fs::write(&tmp, bytes).with_context(|| format!("writing {}", tmp.display()))?;
    fs::rename(&tmp, path).with_context(|| format!("renaming {}", path.display()))?;
    Ok(())
}

fn upsert_claude_section(content: &str) -> String {
    if content.is_empty() {
        return format!("{CLAUDE_MD_SECTION}\n");
    }

    if let (Some(start), Some(end)) = (content.find(SECTION_START), content.find(SECTION_END)) {
        if start < end {
            let section_end = end + SECTION_END.len();
            return join_sections(
                &content[..start],
                CLAUDE_MD_SECTION,
                &content[section_end..],
            );
        }
    }

    if let Some((start, header_len)) = find_unmarked_codegraph_section(content) {
        let after_start = start + header_len;
        let end = content[after_start..]
            .find("\n## ")
            .map(|offset| after_start + offset)
            .unwrap_or(content.len());
        return join_sections(&content[..start], CLAUDE_MD_SECTION, &content[end..]);
    }

    format!("{}\n\n{}\n", content.trim_end(), CLAUDE_MD_SECTION)
}

fn find_unmarked_codegraph_section(content: &str) -> Option<(usize, usize)> {
    if content.starts_with("## CodeGraph") {
        return Some((0, "## CodeGraph".len()));
    }
    content
        .find("\n## CodeGraph")
        .map(|start| (start, "\n## CodeGraph".len()))
}

fn join_sections(before: &str, section: &str, after: &str) -> String {
    let before = before.trim_end_matches('\n');
    let after = after.trim_start_matches('\n');
    match (before.is_empty(), after.is_empty()) {
        (true, true) => format!("{section}\n"),
        (true, false) => format!("{section}\n\n{after}"),
        (false, true) => format!("{before}\n\n{section}\n"),
        (false, false) => format!("{before}\n\n{section}\n\n{after}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn local_install_writes_claude_json_and_claude_md() {
        let dir = TempDir::new().unwrap();
        let project_path = dir.path().canonicalize().unwrap();
        let result = install(&InstallOptions {
            local: true,
            no_init: true,
            project_path: Some(dir.path().to_path_buf()),
            ..Default::default()
        })
        .unwrap();

        assert_eq!(result.claude_json_path, project_path.join(".claude.json"));
        assert!(result.claude_json_changed);
        assert!(result.claude_md_changed);
        assert!(dir.path().join(".claude.json").exists());
        assert!(dir.path().join(".claude").join("CLAUDE.md").exists());
    }

    #[test]
    fn global_install_uses_home_paths() {
        let home = TempDir::new().unwrap();
        let project = TempDir::new().unwrap();
        let result = install(&InstallOptions {
            global: true,
            no_init: true,
            project_path: Some(project.path().to_path_buf()),
            home_dir: Some(home.path().to_path_buf()),
            ..Default::default()
        })
        .unwrap();

        assert_eq!(result.claude_json_path, home.path().join(".claude.json"));
        assert_eq!(
            result.claude_md_path,
            home.path().join(".claude").join("CLAUDE.md")
        );
    }

    #[test]
    fn rejects_multiple_targets() {
        let err = install(&InstallOptions {
            global: true,
            local: true,
            no_init: true,
            ..Default::default()
        })
        .unwrap_err();
        assert!(err.to_string().contains("either --global or --local"));
    }

    #[test]
    fn mcp_config_preserves_existing_servers_and_is_idempotent() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(".claude.json");
        fs::write(
            &path,
            serde_json::to_string_pretty(&json!({
                "mcpServers": { "other": { "command": "other-bin", "args": ["--flag"] } }
            }))
            .unwrap(),
        )
        .unwrap();

        assert!(write_mcp_config(&path).unwrap());
        assert!(!write_mcp_config(&path).unwrap());
        let config: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        let servers = config.get("mcpServers").unwrap();
        assert!(servers.get("other").is_some());
        assert_eq!(
            servers.get("codegraph").unwrap(),
            &json!({"type": "stdio", "command": "cgz", "args": ["serve", "--mcp"]})
        );
    }

    #[test]
    fn permissions_are_explicit_and_preserve_existing_allow_entries() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(".claude").join("settings.json");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            &path,
            serde_json::to_string_pretty(&json!({
                "permissions": { "allow": ["mcp__other__tool"] }
            }))
            .unwrap(),
        )
        .unwrap();

        assert!(write_permissions(&path).unwrap());
        assert!(!write_permissions(&path).unwrap());
        let settings: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        let allow = settings["permissions"]["allow"].as_array().unwrap();
        assert!(allow.iter().any(|v| v == "mcp__other__tool"));
        assert!(allow
            .iter()
            .any(|v| v == "mcp__codegraph__codegraph_status"));
    }

    #[test]
    fn invalid_json_is_not_overwritten() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(".claude.json");
        fs::write(&path, "{").unwrap();

        let err = write_mcp_config(&path).unwrap_err();
        assert!(err.to_string().contains("parsing"));
        assert_eq!(fs::read_to_string(&path).unwrap(), "{");
    }

    #[test]
    fn claude_md_replaces_marked_section_and_preserves_content() {
        let before = "# Project\n\n";
        let old = "<!-- CODEGRAPH_START -->\nold\n<!-- CODEGRAPH_END -->";
        let after = "\n\n## Other\ntext\n";
        let updated = upsert_claude_section(&format!("{before}{old}{after}"));

        assert!(updated.starts_with(before));
        assert!(updated.contains("cgz status"));
        assert!(updated.contains("## Other"));
        assert!(!updated.contains("\nold\n"));
    }

    #[test]
    fn claude_md_replaces_unmarked_codegraph_section() {
        let updated = upsert_claude_section("intro\n\n## CodeGraph\nold\n\n## Next\nkeep\n");

        assert!(updated.contains("intro"));
        assert!(updated.contains("cgz query"));
        assert!(updated.contains("## Next\nkeep"));
        assert!(!updated.contains("\nold\n"));
    }
}
