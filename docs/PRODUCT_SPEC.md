# Agent Context Doctor Product Specification

## Objective

`agent_context_doctor` explains which repository-owned instruction, settings,
and skill files can affect an AI coding agent at a selected path. It addresses
the recurring failure mode where an agent appears to ignore guidance because a
file is misplaced, replaced by an override file, loaded only on demand, guarded
by workspace trust, or outside the selected host's scope.

The tool is an offline, read-only, privacy-safe static resolver. Its output is
evidence for debugging configuration discovery, not a claim about a running
host's complete prompt or semantic instruction conflicts.

## Supported hosts and model

The first release models repository-local discovery for:

| Host | Instructions | Settings | Skills |
| --- | --- | --- | --- |
| Codex | ancestry `AGENTS.override.md` / `AGENTS.md` | ancestry `.codex/config.toml` layers | ancestry `.agents/skills/*/SKILL.md` |
| Claude Code | ancestry and nested `CLAUDE.md`, `CLAUDE.local.md`, and `.claude/CLAUDE.md` | selected working directory `.claude/settings.json` / `settings.local.json` | ancestry and nested `.claude/skills/*/SKILL.md` |
| GitHub Copilot code review | `.github/copilot-instructions.md`, scoped `.github/instructions/**/*.instructions.md`, and `AGENTS.md` | none in v0.1 | none in v0.1 |

`--host copilot` means the GitHub Copilot **code review** surface. It does not
claim to model Copilot CLI, cloud agent, or every IDE, whose supported context
files differ. Rules are versioned as `repository-v0.1`, and JSON reports include
`"copilot_surface": "code_review"`.

## Command

```text
agent_context_doctor audit <PATH> --host codex|claude|copilot|all [--format text|json]
```

Exit codes:

- `0`: resolution completed and no ignored, trust-gated, or out-of-scope files
  were found.
- `1`: resolution completed with at least one diagnostic finding.
- `2`: resolution was incomplete or the input could not be audited safely.

## Resolution states

- `active`: repository-local instructions or settings selected by the static
  model.
- `shadowed`: a Codex `AGENTS.md` is not selected because
  `AGENTS.override.md` exists in the same directory. Config layers are never
  labeled wholly shadowed because closest-wins precedence is key-specific and
  this tool intentionally does not read TOML values.
- `ignored`: a recognized file is in a location the selected host does not load.
- `on-demand`: a discovered skill or path-scoped instruction requires an
  explicit task/path match and is not always loaded.
- `trust-gated`: a Codex project config exists on the selected ancestry, but
  Codex loads project `.codex` layers only for trusted projects. The tool does
  not inspect or infer the user's trust decision.
- `out-of-scope`: an artifact belongs to a different supported host.

Every artifact includes a stable rule identifier and a relative, normalized
path. Text output and JSON must never contain file contents or absolute paths.

## Privacy and safety boundaries

Always:

- audit only the explicit path and its repository ancestry;
- use filesystem metadata and filenames; the only content access is a bounded
  whitespace probe for `AGENTS.override.md`, required because Codex skips empty
  override files;
- keep paths relative to the detected repository root;
- return an incomplete result when traversal, metadata, or limits fail;
- cap ancestry depth, traversal depth, total entries, and artifact count;
- fail closed on non-UTF-8 paths and stop at nested Git repositories.

Never:

- start Codex, Claude Code, Copilot, an editor, or a model;
- read home-directory configuration or environment variables;
- execute hooks, plugins, skills, repository commands, or configuration;
- modify, repair, move, or generate host configuration;
- claim semantic conflicts, effective prompt order, or runtime trust state;
- follow symbolic links while discovering artifacts;
- retain, parse, report, hash, or transmit instruction and settings contents;
- use the network or emit telemetry.

## Testing strategy

- Unit tests cover host parsing, state serialization, relative-path redaction,
  instruction override selection, and deterministic ordering.
- Integration tests use temporary repositories to cover all hosts, misplaced
  files, skills, scoped instructions, symlinks, limits, JSON output, and exit
  codes.
- CI runs formatting, tests, and Clippy on Linux, Windows, and macOS with the
  locked dependency graph.

## Success criteria

- The command produces deterministic text and JSON reports for all supported
  host selectors.
- Reports classify repository-local artifacts into the six documented states.
- No report includes file contents, absolute paths, environment values, or
  unredacted I/O errors.
- The process performs no writes beyond normal process output.
- Documentation does not advertise runtime prompt inspection, semantic conflict
  detection, or automatic repair.

## Deferred work

- User/global configuration and runtime prompt introspection.
- Host-specific semantic parsing of instruction bodies.
- Claude `.claude/rules/*.md`, because distinguishing global rules from
  path-scoped rules requires reading frontmatter.
- Automatic fixes or configuration migration.
- Editor APIs, model APIs, telemetry, and background monitoring.

## Authoritative rule sources

- [Codex `AGENTS.md` discovery](https://developers.openai.com/codex/guides/agents-md)
- [Codex project config precedence and trust](https://developers.openai.com/codex/config-basic)
- [Codex repository skill discovery](https://developers.openai.com/codex/skills)
- [Claude Code instruction discovery](https://code.claude.com/docs/en/memory)
- [Claude Code settings scopes](https://code.claude.com/docs/en/settings)
- [Claude Code skill discovery](https://code.claude.com/docs/en/skills)
- [GitHub Copilot custom-instruction surface support](https://docs.github.com/en/copilot/reference/custom-instructions-support)
