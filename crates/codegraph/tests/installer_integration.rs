use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn cgz_bin() -> PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop();
    path.pop();
    path.push("cgz");
    path
}

fn setup_project(dir: &Path) {
    let src = dir.join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("main.rs"), "fn main() {}\n").unwrap();
}

#[test]
fn install_local_writes_project_claude_files() {
    let dir = TempDir::new().unwrap();
    setup_project(dir.path());

    let output = std::process::Command::new(cgz_bin())
        .args([
            "install",
            "--local",
            "--no-init",
            "--path",
            dir.path().to_str().unwrap(),
        ])
        .output()
        .expect("failed to run cgz install");

    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(dir.path().join(".claude.json").exists());
    assert!(dir.path().join(".claude").join("CLAUDE.md").exists());

    let config: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(dir.path().join(".claude.json")).unwrap())
            .unwrap();
    assert_eq!(
        config["mcpServers"]["codegraph"],
        serde_json::json!({"type": "stdio", "command": "cgz", "args": ["serve", "--mcp"]})
    );
}

#[test]
fn install_global_uses_home_claude_files() {
    let project = TempDir::new().unwrap();
    let home = TempDir::new().unwrap();
    setup_project(project.path());

    let output = std::process::Command::new(cgz_bin())
        .args([
            "install",
            "--global",
            "--no-init",
            "--path",
            project.path().to_str().unwrap(),
        ])
        .env("HOME", home.path())
        .output()
        .expect("failed to run cgz install");

    assert!(output.status.success());
    assert!(home.path().join(".claude.json").exists());
    assert!(home.path().join(".claude").join("CLAUDE.md").exists());
    assert!(!project.path().join(".claude.json").exists());
}

#[test]
fn install_adds_permissions_only_when_requested() {
    let dir = TempDir::new().unwrap();
    setup_project(dir.path());

    let output = std::process::Command::new(cgz_bin())
        .args([
            "install",
            "--local",
            "--no-init",
            "--allow-permissions",
            "--path",
            dir.path().to_str().unwrap(),
        ])
        .output()
        .expect("failed to run cgz install");

    assert!(output.status.success());
    let settings: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(dir.path().join(".claude").join("settings.json")).unwrap(),
    )
    .unwrap();
    let allow = settings["permissions"]["allow"].as_array().unwrap();
    assert!(allow
        .iter()
        .any(|entry| entry == "mcp__codegraph__codegraph_status"));
}

#[test]
fn install_no_init_does_not_initialize_project() {
    let dir = TempDir::new().unwrap();
    setup_project(dir.path());

    let output = std::process::Command::new(cgz_bin())
        .args([
            "install",
            "--local",
            "--no-init",
            "--path",
            dir.path().to_str().unwrap(),
        ])
        .output()
        .expect("failed to run cgz install");

    assert!(output.status.success());
    assert!(!dir.path().join(".codegraph").exists());
}

#[test]
fn install_with_yes_initializes_project() {
    let dir = TempDir::new().unwrap();
    setup_project(dir.path());

    let output = std::process::Command::new(cgz_bin())
        .args([
            "install",
            "--local",
            "--yes",
            "--path",
            dir.path().to_str().unwrap(),
        ])
        .output()
        .expect("failed to run cgz install");

    assert!(output.status.success());
    assert!(dir.path().join(".codegraph").exists());
}
