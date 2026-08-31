use std::fs;
use std::process::Command;

#[test]
fn text_output_uses_documented_state_names_and_clean_exit() {
    let repo = tempfile::tempdir().unwrap();
    fs::create_dir(repo.path().join(".git")).unwrap();
    fs::write(repo.path().join("AGENTS.md"), "private content").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_agent_context_doctor"))
        .args(["audit", repo.path().to_str().unwrap(), "--host", "codex"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("active\tinstructions\tAGENTS.md"));
    assert!(!stdout.contains("private content"));
    assert!(!stdout.contains(repo.path().to_str().unwrap()));
}

#[test]
fn json_diagnostic_and_incomplete_audit_have_stable_exit_codes() {
    let repo = tempfile::tempdir().unwrap();
    fs::create_dir(repo.path().join(".git")).unwrap();
    fs::create_dir(repo.path().join(".codex")).unwrap();
    fs::write(repo.path().join(".codex/config.toml"), "").unwrap();

    let diagnostic = Command::new(env!("CARGO_BIN_EXE_agent_context_doctor"))
        .args([
            "audit",
            repo.path().to_str().unwrap(),
            "--host",
            "codex",
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert_eq!(diagnostic.status.code(), Some(1));
    let report: serde_json::Value = serde_json::from_slice(&diagnostic.stdout).unwrap();
    assert_eq!(report["artifacts"][0]["state"], "trust_gated");

    let missing = repo.path().join("missing");
    let incomplete = Command::new(env!("CARGO_BIN_EXE_agent_context_doctor"))
        .args(["audit", missing.to_str().unwrap(), "--host", "all"])
        .output()
        .unwrap();
    assert_eq!(incomplete.status.code(), Some(2));
    assert_eq!(
        String::from_utf8(incomplete.stderr).unwrap(),
        "agent_context_doctor: audit could not be completed\n"
    );
}

#[test]
fn copilot_root_agents_file_is_not_duplicated_as_out_of_scope() {
    let repo = tempfile::tempdir().unwrap();
    fs::create_dir(repo.path().join(".git")).unwrap();
    fs::write(repo.path().join("AGENTS.md"), "private").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_agent_context_doctor"))
        .args(["audit", repo.path().to_str().unwrap(), "--host", "copilot"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(stdout.matches("AGENTS.md").count(), 1);
}
