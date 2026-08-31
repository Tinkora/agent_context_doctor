use agent_context_doctor::{audit, Host, ResolutionState};
use std::fs;

#[test]
fn classifies_cross_host_repository_context_without_reading_contents() {
    let repo = tempfile::tempdir().unwrap();
    fs::create_dir(repo.path().join(".git")).unwrap();
    fs::create_dir_all(repo.path().join("app/src")).unwrap();
    fs::create_dir_all(repo.path().join(".codex")).unwrap();
    fs::create_dir_all(repo.path().join(".agents/skills/review")).unwrap();
    fs::create_dir_all(repo.path().join(".claude")).unwrap();
    fs::write(repo.path().join("AGENTS.md"), "TOP SECRET INSTRUCTION").unwrap();
    fs::write(repo.path().join("app/AGENTS.md"), "nested secret").unwrap();
    fs::write(repo.path().join(".codex/config.toml"), "secret=true").unwrap();
    fs::write(repo.path().join(".agents/skills/review/SKILL.md"), "secret").unwrap();
    fs::write(repo.path().join(".claude/settings.json"), "{}").unwrap();

    let report = audit(&repo.path().join("app/src"), Host::Codex).unwrap();

    assert!(report.complete);
    assert_eq!(report.model_version, "repository-v0.1");
    assert!(report.artifacts.iter().any(|artifact| {
        artifact.path == "AGENTS.md" && artifact.state == ResolutionState::Active
    }));
    assert!(report.artifacts.iter().any(|artifact| {
        artifact.path == "app/AGENTS.md" && artifact.state == ResolutionState::Active
    }));
    assert!(report.artifacts.iter().any(|artifact| {
        artifact.path == ".codex/config.toml" && artifact.state == ResolutionState::TrustGated
    }));
    assert!(report.artifacts.iter().any(|artifact| {
        artifact.path == ".agents/skills/review/SKILL.md"
            && artifact.state == ResolutionState::OnDemand
    }));
    assert!(report.artifacts.iter().any(|artifact| {
        artifact.path == ".claude/settings.json" && artifact.state == ResolutionState::OutOfScope
    }));
    let json = serde_json::to_string(&report).unwrap();
    assert!(!json.contains(repo.path().to_string_lossy().as_ref()));
    assert!(!json.contains("TOP SECRET"));
}

#[test]
fn recognizes_host_specific_ancestry_and_nested_on_demand_files() {
    let repo = tempfile::tempdir().unwrap();
    fs::create_dir(repo.path().join(".git")).unwrap();
    fs::create_dir_all(repo.path().join("app/src/.agents/skills/local")).unwrap();
    fs::create_dir_all(repo.path().join("app/src/.claude/skills/local")).unwrap();
    fs::create_dir_all(repo.path().join("app/.claude")).unwrap();
    fs::write(repo.path().join("AGENTS.md"), "root").unwrap();
    fs::write(repo.path().join("app/AGENTS.md"), "fallback").unwrap();
    fs::write(repo.path().join("app/AGENTS.override.md"), "override").unwrap();
    fs::write(repo.path().join("CLAUDE.local.md"), "local").unwrap();
    fs::write(repo.path().join("app/.claude/CLAUDE.md"), "project").unwrap();
    fs::write(
        repo.path().join("app/src/.agents/skills/local/SKILL.md"),
        "skill",
    )
    .unwrap();
    fs::write(
        repo.path().join("app/src/.claude/skills/local/SKILL.md"),
        "skill",
    )
    .unwrap();

    let report = audit(&repo.path().join("app/src"), Host::All).unwrap();

    assert!(report.artifacts.iter().any(|artifact| {
        artifact.path == "app/AGENTS.override.md" && artifact.state == ResolutionState::Active
    }));
    assert!(report.artifacts.iter().any(|artifact| {
        artifact.path == "app/AGENTS.md" && artifact.state == ResolutionState::Shadowed
    }));
    assert!(report.artifacts.iter().any(|artifact| {
        artifact.path == "CLAUDE.local.md" && artifact.state == ResolutionState::Active
    }));
    assert!(report.artifacts.iter().any(|artifact| {
        artifact.path == "app/.claude/CLAUDE.md" && artifact.state == ResolutionState::Active
    }));
    assert!(report.artifacts.iter().any(|artifact| {
        artifact.path == "app/src/.agents/skills/local/SKILL.md"
            && artifact.state == ResolutionState::OnDemand
    }));
    assert!(report.artifacts.iter().any(|artifact| {
        artifact.path == "app/src/.claude/skills/local/SKILL.md"
            && artifact.state == ResolutionState::OnDemand
    }));
}

#[test]
fn empty_override_does_not_shadow_agents_instructions() {
    let repo = tempfile::tempdir().unwrap();
    fs::create_dir(repo.path().join(".git")).unwrap();
    fs::write(repo.path().join("AGENTS.md"), "active").unwrap();
    fs::write(repo.path().join("AGENTS.override.md"), " \n\t").unwrap();

    let report = audit(repo.path(), Host::Codex).unwrap();

    assert!(report.artifacts.iter().any(|artifact| {
        artifact.path == "AGENTS.md" && artifact.state == ResolutionState::Active
    }));
    assert!(report.artifacts.iter().any(|artifact| {
        artifact.path == "AGENTS.override.md" && artifact.state == ResolutionState::Ignored
    }));
}

#[test]
fn marks_misplaced_recognized_files_as_ignored() {
    let repo = tempfile::tempdir().unwrap();
    fs::create_dir(repo.path().join(".git")).unwrap();
    fs::create_dir_all(repo.path().join("docs")).unwrap();
    fs::write(repo.path().join("docs/copilot-instructions.md"), "secret").unwrap();
    fs::create_dir_all(repo.path().join(".agents/not_skills/review")).unwrap();
    fs::write(
        repo.path().join(".agents/not_skills/review/SKILL.md"),
        "secret",
    )
    .unwrap();

    let report = audit(repo.path(), Host::Copilot).unwrap();

    assert!(report.artifacts.iter().any(|artifact| {
        artifact.path == "docs/copilot-instructions.md"
            && artifact.state == ResolutionState::Ignored
    }));

    let codex = audit(repo.path(), Host::Codex).unwrap();
    assert!(codex.artifacts.iter().any(|artifact| {
        artifact.path == ".agents/not_skills/review/SKILL.md"
            && artifact.state == ResolutionState::Ignored
    }));
}

#[test]
fn codex_layers_settings_while_claude_uses_the_selected_working_directory() {
    let repo = tempfile::tempdir().unwrap();
    fs::create_dir(repo.path().join(".git")).unwrap();
    fs::create_dir_all(repo.path().join("app/.codex")).unwrap();
    fs::create_dir_all(repo.path().join(".codex")).unwrap();
    fs::create_dir_all(repo.path().join("app/.claude")).unwrap();
    fs::create_dir_all(repo.path().join(".claude")).unwrap();
    fs::create_dir_all(repo.path().join("other/.codex")).unwrap();
    fs::write(repo.path().join(".codex/config.toml"), "").unwrap();
    fs::write(repo.path().join("app/.codex/config.toml"), "").unwrap();
    fs::write(repo.path().join(".claude/settings.json"), "{}").unwrap();
    fs::write(repo.path().join("app/.claude/settings.json"), "{}").unwrap();
    fs::write(repo.path().join("app/.claude/settings.local.json"), "{}").unwrap();
    fs::write(repo.path().join("other/.codex/config.toml"), "").unwrap();

    let report = audit(repo.path().join("app").as_path(), Host::Codex).unwrap();

    assert!(report.artifacts.iter().any(|artifact| {
        artifact.path == ".codex/config.toml" && artifact.state == ResolutionState::TrustGated
    }));
    assert!(report.artifacts.iter().any(|artifact| {
        artifact.path == "app/.codex/config.toml" && artifact.state == ResolutionState::TrustGated
    }));
    assert!(report.artifacts.iter().any(|artifact| {
        artifact.path == "other/.codex/config.toml" && artifact.state == ResolutionState::Ignored
    }));
    assert!(report
        .artifacts
        .iter()
        .filter(|artifact| artifact.host == Host::Claude)
        .all(|artifact| artifact.state == ResolutionState::OutOfScope));

    let claude = audit(repo.path().join("app").as_path(), Host::Claude).unwrap();
    assert!(claude.artifacts.iter().any(|artifact| {
        artifact.path == "app/.claude/settings.json" && artifact.state == ResolutionState::Active
    }));
    assert!(claude.artifacts.iter().any(|artifact| {
        artifact.path == "app/.claude/settings.local.json"
            && artifact.state == ResolutionState::Active
    }));
    assert!(claude.artifacts.iter().any(|artifact| {
        artifact.path == ".claude/settings.json" && artifact.state == ResolutionState::Ignored
    }));
}

#[cfg(unix)]
#[test]
fn does_not_follow_symlinked_context_files() {
    use std::os::unix::fs::symlink;

    let repo = tempfile::tempdir().unwrap();
    fs::create_dir(repo.path().join(".git")).unwrap();
    let outside = tempfile::NamedTempFile::new().unwrap();
    symlink(outside.path(), repo.path().join("AGENTS.md")).unwrap();

    let report = audit(repo.path(), Host::Codex).unwrap();

    assert!(report.artifacts.iter().any(|artifact| {
        artifact.path == "AGENTS.md" && artifact.state == ResolutionState::Ignored
    }));
}

#[test]
fn copilot_selector_is_explicitly_the_code_review_surface() {
    let repo = tempfile::tempdir().unwrap();
    fs::create_dir(repo.path().join(".git")).unwrap();
    fs::write(repo.path().join("AGENTS.md"), "private").unwrap();

    let report = audit(repo.path(), Host::Copilot).unwrap();

    assert_eq!(report.copilot_surface, "code_review");
    assert!(report.artifacts.iter().any(|artifact| {
        artifact.host == Host::Copilot
            && artifact.path == "AGENTS.md"
            && artifact.state == ResolutionState::Active
    }));
}

#[test]
fn does_not_cross_into_nested_git_repositories() {
    let repo = tempfile::tempdir().unwrap();
    fs::create_dir(repo.path().join(".git")).unwrap();
    fs::create_dir_all(repo.path().join("vendor/nested/.git")).unwrap();
    fs::write(repo.path().join("vendor/nested/AGENTS.md"), "nested").unwrap();

    let report = audit(repo.path(), Host::All).unwrap();

    assert!(report.artifacts.is_empty());
}

#[cfg(target_os = "linux")]
#[test]
fn rejects_non_utf8_repository_paths_instead_of_omitting_them() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let repo = tempfile::tempdir().unwrap();
    fs::create_dir(repo.path().join(".git")).unwrap();
    fs::write(repo.path().join(OsString::from_vec(vec![0xff])), "private").unwrap();

    assert!(audit(repo.path(), Host::All).is_err());
}
