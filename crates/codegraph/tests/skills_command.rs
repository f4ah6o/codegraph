use std::process::Command;

#[test]
fn cli_skills_prints_agent_playbook_without_project() {
    let bin = env!("CARGO_BIN_EXE_cgz");
    let output = Command::new(bin).arg("skills").output().unwrap();

    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("# cgz CodeGraph Skill"), "{text}");
    assert!(text.contains("cgz status --path <project>"), "{text}");
    assert!(text.contains("cgz context --path <project>"), "{text}");
    assert!(text.contains("cgz affected --path <project>"), "{text}");
    assert!(
        text.contains("Treat all `cgz` output as navigation evidence"),
        "{text}"
    );
}
