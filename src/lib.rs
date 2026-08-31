use anyhow::{bail, Context, Result};
use clap::ValueEnum;
use serde::Serialize;
use std::collections::BTreeSet;
use std::fmt;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Component, Path, PathBuf};

const MAX_ANCESTORS: usize = 64;
const MAX_ARTIFACTS: usize = 4096;
const MAX_TREE_DEPTH: usize = 64;
const MAX_TREE_ENTRIES: usize = 16_384;
const MAX_WHITESPACE_PROBE_BYTES: usize = 65_536;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum Host {
    Codex,
    Claude,
    Copilot,
    All,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionState {
    Active,
    Shadowed,
    Ignored,
    OnDemand,
    TrustGated,
    OutOfScope,
}

impl fmt::Display for ResolutionState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Active => "active",
            Self::Shadowed => "shadowed",
            Self::Ignored => "ignored",
            Self::OnDemand => "on_demand",
            Self::TrustGated => "trust_gated",
            Self::OutOfScope => "out_of_scope",
        };
        formatter.write_str(value)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Artifact {
    pub host: Host,
    pub kind: &'static str,
    pub path: String,
    pub state: ResolutionState,
    pub rule_id: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Report {
    pub model_version: &'static str,
    pub copilot_surface: &'static str,
    pub complete: bool,
    pub artifacts: Vec<Artifact>,
}

pub fn audit(target: &Path, selected: Host) -> Result<Report> {
    let target = target
        .canonicalize()
        .with_context(|| "selected path is unavailable")?;
    let start = if target.is_dir() {
        target.clone()
    } else {
        target
            .parent()
            .context("selected path has no parent")?
            .to_path_buf()
    };
    let root = find_repository_root(&start)?;
    let ancestry = repository_ancestry(&root, &start)?;
    let ancestry_set: BTreeSet<PathBuf> = ancestry.iter().cloned().collect();
    let mut scan = ScanState {
        artifacts: Vec::new(),
        visited: 0,
    };

    collect_tree(&root, &root, &start, selected, &ancestry_set, &mut scan, 0)?;
    let mut artifacts = scan.artifacts;
    classify_codex_instruction_overrides(&mut artifacts);
    artifacts.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then(left.host_name().cmp(right.host_name()))
            .then(left.state.cmp(&right.state))
    });

    Ok(Report {
        model_version: "repository-v0.1",
        copilot_surface: "code_review",
        complete: true,
        artifacts,
    })
}

struct ScanState {
    artifacts: Vec<Artifact>,
    visited: usize,
}

impl Artifact {
    fn host_name(&self) -> &'static str {
        match self.host {
            Host::Codex => "codex",
            Host::Claude => "claude",
            Host::Copilot => "copilot",
            Host::All => "all",
        }
    }
}

fn find_repository_root(start: &Path) -> Result<PathBuf> {
    let mut current = start;
    for _ in 0..MAX_ANCESTORS {
        let marker = current.join(".git");
        if fs::symlink_metadata(&marker).is_ok() {
            return Ok(current.to_path_buf());
        }
        current = current
            .parent()
            .context("selected path is not inside a Git repository")?;
    }
    bail!("repository ancestry limit exceeded")
}

fn repository_ancestry(root: &Path, target: &Path) -> Result<Vec<PathBuf>> {
    let relative = target
        .strip_prefix(root)
        .context("selected path escaped the repository")?;
    let mut result = vec![root.to_path_buf()];
    let mut current = root.to_path_buf();
    for component in relative.components() {
        if let Component::Normal(part) = component {
            current.push(part);
            result.push(current.clone());
        }
    }
    if result.len() > MAX_ANCESTORS {
        bail!("repository ancestry limit exceeded");
    }
    Ok(result)
}

fn collect_tree(
    root: &Path,
    directory: &Path,
    target: &Path,
    selected: Host,
    ancestry: &BTreeSet<PathBuf>,
    scan: &mut ScanState,
    depth: usize,
) -> Result<()> {
    if depth > MAX_TREE_DEPTH {
        bail!("repository traversal depth exceeded");
    }
    let entries = fs::read_dir(directory).with_context(|| "repository traversal failed")?;
    for entry in entries {
        let entry = entry.with_context(|| "repository traversal failed")?;
        scan.visited += 1;
        if scan.visited > MAX_TREE_ENTRIES {
            bail!("repository entry limit exceeded");
        }
        if entry.file_name().to_str().is_none() {
            bail!("repository contains a non-UTF-8 path");
        }
        let path = entry.path();
        let relative = path
            .strip_prefix(root)
            .context("repository traversal escaped root")?;
        if relative == Path::new(".git") {
            continue;
        }
        let metadata = fs::symlink_metadata(&path).with_context(|| "artifact metadata failed")?;
        if metadata.file_type().is_symlink() {
            if let Some(mut artifact) = classify(root, target, &path, ancestry) {
                artifact.state = if selected != Host::All && selected != artifact.host {
                    ResolutionState::OutOfScope
                } else {
                    ResolutionState::Ignored
                };
                scan.artifacts.push(artifact);
                if scan.artifacts.len() > MAX_ARTIFACTS {
                    bail!("artifact limit exceeded");
                }
            }
            continue;
        }
        if metadata.is_dir() {
            if path != root && fs::symlink_metadata(path.join(".git")).is_ok() {
                continue;
            }
            collect_tree(root, &path, target, selected, ancestry, scan, depth + 1)?;
            continue;
        }
        let file_name = path.file_name().and_then(|value| value.to_str());
        let override_has_content = if file_name == Some("AGENTS.override.md") {
            Some(has_non_whitespace(&path)?)
        } else {
            None
        };
        let suppress_shared_codex_artifact =
            selected == Host::Copilot && file_name == Some("AGENTS.md");
        if !suppress_shared_codex_artifact {
            if let Some(mut artifact) = classify(root, target, &path, ancestry) {
                if override_has_content == Some(false) {
                    artifact.state = ResolutionState::Ignored;
                }
                if selected != Host::All && selected != artifact.host {
                    artifact.state = ResolutionState::OutOfScope;
                }
                scan.artifacts.push(artifact);
                if scan.artifacts.len() > MAX_ARTIFACTS {
                    bail!("artifact limit exceeded");
                }
            }
        }
        if matches!(selected, Host::All | Host::Copilot) {
            if let Some(artifact) = classify_copilot_code_review(root, target, &path) {
                scan.artifacts.push(artifact);
                if scan.artifacts.len() > MAX_ARTIFACTS {
                    bail!("artifact limit exceeded");
                }
            }
        }
    }
    Ok(())
}

fn has_non_whitespace(path: &Path) -> Result<bool> {
    let mut file = File::open(path).with_context(|| "override probe failed")?;
    let mut buffer = [0_u8; 4096];
    let mut total = 0;
    loop {
        let read = file
            .read(&mut buffer)
            .with_context(|| "override probe failed")?;
        if read == 0 {
            return Ok(false);
        }
        if buffer[..read]
            .iter()
            .any(|byte| !byte.is_ascii_whitespace())
        {
            return Ok(true);
        }
        total += read;
        if total >= MAX_WHITESPACE_PROBE_BYTES {
            bail!("override whitespace probe limit exceeded");
        }
    }
}

fn classify_copilot_code_review(root: &Path, _target: &Path, path: &Path) -> Option<Artifact> {
    if path.file_name()?.to_str()? != "AGENTS.md" {
        return None;
    }
    let relative = normalized_relative(root, path)?;
    let parent = path.parent()?;
    Some(artifact(
        Host::Copilot,
        "instructions",
        relative,
        if parent == root {
            ResolutionState::Active
        } else {
            ResolutionState::Ignored
        },
        "copilot.code_review.agent_instructions",
    ))
}

fn classify(
    root: &Path,
    target: &Path,
    path: &Path,
    ancestry: &BTreeSet<PathBuf>,
) -> Option<Artifact> {
    let relative = normalized_relative(root, path)?;
    let parent = path.parent()?;
    let name = path.file_name()?.to_str()?;

    if name == "AGENTS.md" || name == "AGENTS.override.md" {
        return Some(artifact(
            Host::Codex,
            "instructions",
            relative,
            if ancestry.contains(parent) {
                ResolutionState::Active
            } else {
                ResolutionState::Ignored
            },
            "codex.instructions.ancestry",
        ));
    }
    if name == "CLAUDE.md" || name == "CLAUDE.local.md" {
        let scope_directory =
            if parent.file_name().and_then(|value| value.to_str()) == Some(".claude") {
                parent.parent()?
            } else {
                parent
            };
        let state = if ancestry.contains(scope_directory) {
            ResolutionState::Active
        } else if scope_directory.starts_with(target) {
            ResolutionState::OnDemand
        } else {
            ResolutionState::Ignored
        };
        return Some(artifact(
            Host::Claude,
            "instructions",
            relative,
            state,
            "claude.instructions.ancestry",
        ));
    }
    if relative.ends_with(".codex/config.toml") {
        let settings_scope = parent.parent()?;
        return Some(artifact(
            Host::Codex,
            "settings",
            relative,
            if ancestry.contains(settings_scope) {
                ResolutionState::TrustGated
            } else {
                ResolutionState::Ignored
            },
            "codex.settings.project_trust",
        ));
    }
    if relative.ends_with(".claude/settings.json")
        || relative.ends_with(".claude/settings.local.json")
    {
        let settings_scope = parent.parent()?;
        return Some(artifact(
            Host::Claude,
            "settings",
            relative,
            if settings_scope == target {
                ResolutionState::Active
            } else {
                ResolutionState::Ignored
            },
            "claude.settings.primary_working_directory",
        ));
    }
    if let Some(scope) = skill_scope(&relative, &[".agents", "skills"]) {
        let scope = root.join(scope);
        return Some(artifact(
            Host::Codex,
            "skill",
            relative,
            if ancestry.contains(&scope) {
                ResolutionState::OnDemand
            } else {
                ResolutionState::Ignored
            },
            "codex.skills.on_demand",
        ));
    }
    if let Some(scope) = skill_scope(&relative, &[".claude", "skills"]) {
        let scope = root.join(scope);
        return Some(artifact(
            Host::Claude,
            "skill",
            relative,
            if ancestry.contains(&scope) || scope.starts_with(target) {
                ResolutionState::OnDemand
            } else {
                ResolutionState::Ignored
            },
            "claude.skills.on_demand",
        ));
    }
    if name == "SKILL.md" && has_path_segment(&relative, ".agents") {
        return Some(artifact(
            Host::Codex,
            "skill",
            relative,
            ResolutionState::Ignored,
            "codex.skills.location",
        ));
    }
    if name == "SKILL.md" && has_path_segment(&relative, ".claude") {
        return Some(artifact(
            Host::Claude,
            "skill",
            relative,
            ResolutionState::Ignored,
            "claude.skills.location",
        ));
    }
    if relative == ".github/copilot-instructions.md" {
        return Some(artifact(
            Host::Copilot,
            "instructions",
            relative,
            ResolutionState::Active,
            "copilot.instructions.repository",
        ));
    }
    if relative.starts_with(".github/instructions/") && relative.ends_with(".instructions.md") {
        return Some(artifact(
            Host::Copilot,
            "instructions",
            relative,
            ResolutionState::OnDemand,
            "copilot.instructions.path_scoped",
        ));
    }
    if name == "copilot-instructions.md" || name.ends_with(".instructions.md") {
        return Some(artifact(
            Host::Copilot,
            "instructions",
            relative,
            ResolutionState::Ignored,
            "copilot.instructions.location",
        ));
    }
    None
}

fn has_path_segment(relative: &str, expected: &str) -> bool {
    relative.split('/').any(|part| part == expected)
}

fn skill_scope(relative: &str, sequence: &[&str]) -> Option<PathBuf> {
    if !relative.ends_with("/SKILL.md") {
        return None;
    }
    let parts: Vec<_> = relative.split('/').collect();
    let marker = parts
        .windows(sequence.len())
        .position(|window| window == sequence)?;
    if parts.len() <= marker + sequence.len() + 1 {
        return None;
    }
    Some(parts[..marker].iter().collect())
}

fn normalized_relative(root: &Path, path: &Path) -> Option<String> {
    let relative = path.strip_prefix(root).ok()?;
    let parts: Option<Vec<_>> = relative
        .components()
        .map(|component| match component {
            Component::Normal(value) => value.to_str(),
            _ => None,
        })
        .collect();
    Some(parts?.join("/"))
}

fn artifact(
    host: Host,
    kind: &'static str,
    path: String,
    state: ResolutionState,
    rule_id: &'static str,
) -> Artifact {
    Artifact {
        host,
        kind,
        path,
        state,
        rule_id,
    }
}

fn classify_codex_instruction_overrides(artifacts: &mut [Artifact]) {
    let override_directories: BTreeSet<String> = artifacts
        .iter()
        .filter(|artifact| {
            artifact.host == Host::Codex
                && artifact.path.ends_with("AGENTS.override.md")
                && artifact.state == ResolutionState::Active
        })
        .filter_map(|artifact| {
            Path::new(&artifact.path)
                .parent()
                .map(|path| path.to_string_lossy().into_owned())
        })
        .collect();
    for artifact in artifacts {
        if artifact.host != Host::Codex
            || !artifact.path.ends_with("AGENTS.md")
            || artifact.path.ends_with("AGENTS.override.md")
            || artifact.state != ResolutionState::Active
        {
            continue;
        }
        let parent = Path::new(&artifact.path)
            .parent()
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_default();
        if override_directories.contains(&parent) {
            artifact.state = ResolutionState::Shadowed;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_and_state_use_stable_json_names() {
        assert_eq!(
            serde_json::to_string(&Host::Copilot).unwrap(),
            "\"copilot\""
        );
        assert_eq!(
            serde_json::to_string(&ResolutionState::TrustGated).unwrap(),
            "\"trust_gated\""
        );
    }
}
