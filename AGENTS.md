# AGENTS.md

## Work Style

- Keep changes surgical. Match the current small-Rust-CLI shape.
- Do not add new dependencies unless the standard library or existing crates are clearly insufficient.
- Preserve text output and JSON output contracts when changing command behavior.

## Changelog

- Keep `CHANGELOG.md` updated for user-visible changes, packaging changes, and release automation changes.
- Follow `~/git/twopy/CHANGELOG.md` format:
  - `# Changelog`
  - `## Unreleased`
  - dated release headings like `## 0.1.1 - 2026-04-25`
  - sections: `### Features`, `### Changes`, `### Fixes`
- Add entries under `Unreleased` while developing.
- Move entries from `Unreleased` into a dated release heading when tagging.
- Keep entries concise, past-tense, user-facing, and grounded in behavior.

## Rust Checks

- Before handoff after code changes, run:
  - `cargo fmt --check`
  - `cargo clippy --locked --all-targets --all-features -- -D warnings`
  - `cargo test --locked`
- For docs-only changes, `git diff --check` is enough.

## Release

- Version tags are `vX.Y.Z`.
- Before tagging, update `Cargo.toml` version and move changelog entries into the release heading.
- Tag pushes run the release workflow, which checks the Cargo version, runs locked tests, publishes the GitHub release, and updates `gumadeiras/homebrew-tap`.
- Do not publish or tag without explicit user approval.

## Zotero Behavior

- Keep local Zotero desktop mode read-only unless explicitly asked to add write support.
- Keep local API failures actionable; mention Zotero desktop API access when relevant.
- Treat JSON input precedence as part of the CLI contract: `--value`, stdin marker `-`, existing file path, then inline JSON.
