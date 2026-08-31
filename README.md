# Agent Context Doctor

[简体中文](README.zh-CN.md)

`agent_context_doctor` is an offline, read-only CLI that explains how
repository-local instructions, settings, and skills are discovered for Codex,
Claude Code, and GitHub Copilot.

Use it when an agent appears to ignore repository guidance and you need a
privacy-safe inventory of files that are active, shadowed, ignored, on demand,
trust gated, or outside the selected host's scope.

## Why this exists

Agent hosts use different filenames, directories, ancestry rules, and loading
conditions. A valid file in the wrong location can silently have no effect,
while a project settings file may be gated by workspace trust. This tool makes
that static discovery model reviewable without starting a host or sending any
repository data to a model.

## Install

Download a release archive for your platform, or build with Rust 1.85 or newer:

```bash
cargo install --path . --locked
```

## Usage

```bash
agent_context_doctor audit . --host all
agent_context_doctor audit crates/core --host codex --format json
```

The audit is limited to the explicitly selected Git repository. Reports contain
normalized relative paths and stable rule identifiers, never file contents or
absolute paths.

Exit codes:

- `0`: resolution completed without diagnostic findings;
- `1`: resolution completed with ignored, shadowed, trust-gated, or out-of-scope
  artifacts;
- `2`: the audit could not be completed safely.

## Supported repository-local files

| Host | Instructions | Settings | Skills |
| --- | --- | --- | --- |
| Codex | ancestry `AGENTS.override.md` / `AGENTS.md` | ancestry `.codex/config.toml` | ancestry `.agents/skills/*/SKILL.md` |
| Claude Code | ancestry and nested `CLAUDE.md` variants | selected working directory settings | ancestry and nested `.claude/skills/*/SKILL.md` |
| GitHub Copilot code review | repository, path-scoped, and `AGENTS.md` instructions | not modeled | not modeled |

`--host copilot` is deliberately limited to the GitHub Copilot code review
surface. The report identifies its rules as `repository-v0.1`. Host behavior
evolves; open an issue with an authoritative source when a rule needs updating.

## Non-goals

The tool does not parse, retain, or report instruction or settings contents. Its
only content access is a bounded whitespace probe because Codex skips empty
`AGENTS.override.md` files. It does not inspect home-directory configuration,
run a host or model, infer semantic conflicts or runtime prompt order, decide
workspace trust, repair files, use the network, or emit telemetry.
See the [product specification](docs/PRODUCT_SPEC.md) for the exact contract.

## Community and security

- [Contributing](CONTRIBUTING.md)
- [Security policy](SECURITY.md)
- [Support](SUPPORT.md)
- [Code of conduct](CODE_OF_CONDUCT.md)

If this project saves you time, you can support continued maintenance through
the funding link shown by GitHub.
