use crate::config::CodeGraphConfig;
use crate::extraction::should_include_file;
use crate::{find_nearest_codegraph_root, CodeGraph, CODEGRAPH_DIR};
use anyhow::{anyhow, Result};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

const DEFAULT_DEBOUNCE_MS: u64 = 300;

pub struct WatcherConfig {
    pub debounce_ms: u64,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            debounce_ms: DEFAULT_DEBOUNCE_MS,
        }
    }
}

pub fn run_watcher(root: PathBuf, watcher_config: WatcherConfig) -> Result<()> {
    let cg_root = find_nearest_codegraph_root(&root)
        .ok_or_else(|| anyhow!("CodeGraph not initialized in {}", root.display()))?;
    let cg = CodeGraph::open(&cg_root)?;
    let config = cg.config().clone();
    drop(cg);

    let (tx, rx) = mpsc::channel();

    let mut watcher = RecommendedWatcher::new(
        move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                let _ = tx.send(event);
            }
        },
        Config::default(),
    )?;

    watcher.watch(&cg_root, RecursiveMode::Recursive)?;

    let mut pending_paths: BTreeSet<PathBuf> = BTreeSet::new();
    let debounce = Duration::from_millis(watcher_config.debounce_ms);

    eprintln!(
        "Watching {} (debounce {}ms)...",
        cg_root.display(),
        watcher_config.debounce_ms
    );
    eprintln!("Press Ctrl+C to stop.");

    loop {
        match rx.recv_timeout(debounce) {
            Ok(event) => {
                if is_relevant_event(&event, &cg_root, &config) {
                    for path in &event.paths {
                        let rel = path.strip_prefix(&cg_root).unwrap_or(path).to_path_buf();
                        if should_watch_path(&rel, &config) {
                            pending_paths.insert(path.clone());
                        }
                    }
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if !pending_paths.is_empty() {
                    let paths = std::mem::take(&mut pending_paths);
                    match sync_if_relevant_changes(&cg_root, &config, &paths) {
                        Ok(count) => {
                            if count > 0 {
                                eprintln!("Synced {} changed file(s)", count);
                            }
                        }
                        Err(err) => {
                            eprintln!("Sync error: {err}");
                        }
                    }
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                break;
            }
        }
    }

    Ok(())
}

fn is_relevant_event(event: &Event, root: &std::path::Path, config: &CodeGraphConfig) -> bool {
    match event.kind {
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {}
        _ => return false,
    }

    for path in &event.paths {
        let rel = match path.strip_prefix(root) {
            Ok(r) => r.to_path_buf(),
            Err(_) => continue,
        };
        if should_watch_path(&rel, config) {
            return true;
        }
    }
    false
}

pub fn should_watch_path(rel: &std::path::Path, config: &CodeGraphConfig) -> bool {
    if rel.components().any(|c| c.as_os_str() == CODEGRAPH_DIR) {
        return false;
    }
    should_include_file(rel, config)
}

fn sync_if_relevant_changes(
    root: &std::path::Path,
    config: &CodeGraphConfig,
    changed_paths: &BTreeSet<PathBuf>,
) -> Result<usize> {
    let mut relevant: BTreeSet<PathBuf> = BTreeSet::new();
    for path in changed_paths {
        let rel = path.strip_prefix(root).unwrap_or(path).to_path_buf();
        if should_watch_path(&rel, config) {
            relevant.insert(rel);
        }
    }
    if relevant.is_empty() {
        return Ok(0);
    }

    let mut cg = CodeGraph::open(root)?;
    let result = cg.sync()?;
    Ok((result.files_indexed + result.files_deleted) as usize)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::CodeGraphConfig;

    #[test]
    fn test_should_watch_path_excludes_codegraph_dir() {
        let config = CodeGraphConfig::default_for_root(".");
        assert!(!should_watch_path(
            std::path::Path::new(".codegraph/codegraph.db"),
            &config
        ));
        assert!(!should_watch_path(
            std::path::Path::new(".codegraph/config.json"),
            &config
        ));
    }

    #[test]
    fn test_should_watch_path_excludes_build_outputs() {
        let config = CodeGraphConfig::default_for_root(".");
        assert!(!should_watch_path(
            std::path::Path::new("target/debug/main"),
            &config
        ));
        assert!(!should_watch_path(
            std::path::Path::new("build/output.js"),
            &config
        ));
        assert!(!should_watch_path(
            std::path::Path::new("dist/bundle.js"),
            &config
        ));
    }

    #[test]
    fn test_should_watch_path_includes_source_files() {
        let config = CodeGraphConfig::default_for_root(".");
        assert!(should_watch_path(
            std::path::Path::new("src/main.rs"),
            &config
        ));
        assert!(should_watch_path(
            std::path::Path::new("lib/app.ts"),
            &config
        ));
        assert!(should_watch_path(
            std::path::Path::new("src/lib.mbt"),
            &config
        ));
    }

    #[test]
    fn test_should_watch_path_excludes_non_included_files() {
        let config = CodeGraphConfig::default_for_root(".");
        assert!(!should_watch_path(
            std::path::Path::new("README.md"),
            &config
        ));
        assert!(!should_watch_path(
            std::path::Path::new("image.png"),
            &config
        ));
    }

    #[test]
    fn test_watcher_config_default_debounce() {
        let config = WatcherConfig::default();
        assert_eq!(config.debounce_ms, 300);
    }
}
