use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn cgz() -> String {
    env!("CARGO_BIN_EXE_cgz").to_string()
}

fn run(args: &[&str]) -> std::process::Output {
    Command::new(cgz()).args(args).output().unwrap()
}

fn assert_success(output: &std::process::Output) {
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn cli_index_shows_human_readable_output() {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("src")).unwrap();
    fs::write(dir.path().join("src/lib.rs"), "pub fn hello() {}\n").unwrap();

    assert_success(&run(&["init", dir.path().to_str().unwrap()]));

    let output = run(&["index", dir.path().to_str().unwrap()]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();

    assert!(output.status.success(), "stderr: {stderr}");
    assert!(
        stdout.contains("Indexed 1 file, 2 nodes, 1 edge in "),
        "expected human-readable index summary, got: {stdout}"
    );
    assert!(
        !stdout.contains("skipped 0") && !stdout.contains("deleted 0"),
        "expected zero-count details to stay out of the summary, got: {stdout}"
    );
    assert!(
        stderr.contains("Indexing started"),
        "expected progress start message on stderr, got: {stderr}"
    );
}

#[test]
fn cli_sync_shows_human_readable_output() {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("src")).unwrap();
    fs::write(dir.path().join("src/lib.rs"), "pub fn sync_test() {}\n").unwrap();

    assert_success(&run(&["init", dir.path().to_str().unwrap(), "-i"]));

    let output = run(&["sync", dir.path().to_str().unwrap()]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();

    assert!(output.status.success(), "stderr: {stderr}");
    assert!(
        stdout.contains("Indexed 0 files, 1 file skipped, 0 nodes, 1 edge in "),
        "expected human-readable sync summary, got: {stdout}"
    );
    assert!(
        stderr.contains("Syncing started"),
        "expected progress start message on stderr, got: {stderr}"
    );
}

#[test]
fn cli_index_quiet_suppresses_output() {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("src")).unwrap();
    fs::write(dir.path().join("src/lib.rs"), "pub fn quiet_fn() {}\n").unwrap();

    assert_success(&run(&["init", dir.path().to_str().unwrap()]));

    let output = run(&["index", dir.path().to_str().unwrap(), "--quiet"]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();

    assert!(
        stdout.is_empty(),
        "expected no stdout in quiet mode, got: {stdout}"
    );
    assert!(
        !stderr.contains("Indexing started"),
        "expected no progress start message in quiet mode, got: {stderr}"
    );
}

#[test]
fn cli_status_json_remains_machine_readable() {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("src")).unwrap();
    fs::write(dir.path().join("src/lib.rs"), "pub fn status_fn() {}\n").unwrap();

    assert_success(&run(&["init", dir.path().to_str().unwrap(), "-i"]));

    let output = run(&["status", dir.path().to_str().unwrap(), "--json"]);
    assert!(output.status.success());
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("status --json should produce valid JSON");
    assert!(json["file_count"].as_i64().unwrap_or(0) >= 1);
}

#[test]
fn cli_index_shows_parse_errors() {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("src")).unwrap();
    fs::write(dir.path().join("src/lib.rs"), "fn broken( {\n").unwrap();

    assert_success(&run(&["init", dir.path().to_str().unwrap()]));

    let output = run(&["index", dir.path().to_str().unwrap()]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();

    assert!(!output.status.success());
    assert!(
        stdout.contains("1 file errored"),
        "expected errored count in summary, got: {stdout}"
    );
    assert!(
        stderr.contains("[parse] src/lib.rs: could not parse rust syntax"),
        "expected categorized parse error, got: {stderr}"
    );
}

#[test]
fn cli_init_index_exits_nonzero_on_index_errors() {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("src")).unwrap();
    fs::write(dir.path().join("src/lib.rs"), "fn broken( {\n").unwrap();

    let output = run(&["init", dir.path().to_str().unwrap(), "-i"]);
    let stderr = String::from_utf8(output.stderr).unwrap();

    assert!(!output.status.success());
    assert!(
        stderr.contains("Indexing started") && stderr.contains("[parse] src/lib.rs"),
        "expected init -i to report categorized index failure, got: {stderr}"
    );
}

#[test]
fn cli_index_shows_unsupported_errors() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("README"), "plain text\n").unwrap();

    assert_success(&run(&["init", dir.path().to_str().unwrap()]));
    let config_path = dir.path().join(".codegraph").join("config.json");
    let mut config: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&config_path).unwrap()).unwrap();
    config["include"] = serde_json::json!(["README"]);
    fs::write(
        &config_path,
        serde_json::to_string_pretty(&config).unwrap() + "\n",
    )
    .unwrap();

    let output = run(&["index", dir.path().to_str().unwrap()]);
    let stderr = String::from_utf8(output.stderr).unwrap();

    assert!(!output.status.success());
    assert!(
        stderr.contains("[unsupported] README: unsupported file type"),
        "expected categorized unsupported error, got: {stderr}"
    );
}
