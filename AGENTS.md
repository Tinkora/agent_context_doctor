# Repository Guide for AI Agents

## Product boundary

This repository provides an offline, read-only static resolver for
repository-local AI agent context files. Preserve the v0.1 boundary in
`docs/PRODUCT_SPEC.md`: do not start hosts or models, inspect home-directory
configuration, parse or expose artifact contents, infer semantic conflicts or
runtime trust, modify configuration, use the network, emit telemetry, or add a
background service. Preserve the bounded whitespace-only probe for
`AGENTS.override.md`; do not widen content access.

Reports must not expose absolute paths, file contents, environment values, or
raw I/O errors. Treat filenames and filesystem metadata as untrusted input.
Never turn a resource-limit or traversal failure into a clean result.

## Conventions

- Use English Conventional Commits.
- Write code and code comments in English.
- Keep the default README in English with a Chinese entry point.
- Keep English and Chinese documentation behaviorally aligned.
- Use underscores for project-owned filenames and identifiers where external
  formats do not require another spelling.

## Required checks

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```
