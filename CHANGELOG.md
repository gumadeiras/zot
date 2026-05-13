# Changelog

## Unreleased

### Changes

- Added tag-driven release automation that runs locked tests, creates GitHub releases, and updates the Homebrew tap.
- Trimmed Tokio features to the current-thread runtime needed by the CLI.

### Fixes

- Fixed local Zotero PDF downloads for `file://` attachment URLs while keeping HTTP attachment downloads unchanged.
- Kept the current codebase clean under clippy.

## 0.1.1 - 2026-04-25

### Changes

- Added Cargo package metadata for Homebrew packaging.
- Documented Homebrew tap installation through `gumadeiras/tap`.
- Bumped the package version so the tap can pin a tagged release.

## 0.1.0 - 2026-04-05

Initial release.

### Features

- Added an installable Rust CLI for Zotero with typed Web API access, config parsing, and `search`, `collections`, and `item` commands.
- Added support for user, group, username-resolved, and local Zotero desktop libraries.
- Added username-to-user-id resolution from public Zotero profiles.
- Added `open`, `pdf`, and `add` item actions for opening item URLs, resolving PDF attachments, downloading PDFs, and building new item payloads from DOI, ISBN, URL, or raw JSON input.
- Added local Zotero desktop API mode through `ZOTERO_LOCAL=1` and `--local` for read-only workflows without a web API key.
- Added text and JSON output modes, including structured JSON responses for action commands.
- Added `zot add json` support for stdin, file path, inline JSON, and explicit `--value` item input.
- Added dry-run JSON validation for item creation without requiring library credentials.

### Fixes

- Accepted Zotero API fields that may be returned as either strings or `false`, fixing collection decoding against real libraries.
- Displayed DOI values returned under uppercase `DOI` item fields.
- Preferred real file paths over inline JSON detection for `zot add json`, including paths that begin with `[` or `{`.
- Reported local Zotero API setup failures with actionable guidance and kept write commands unsupported in local mode.

### Changes

- Documented 1Password-based API key usage and local Zotero desktop examples.
- Added parser and binary-level regression coverage for `zot add json` stdin, file, inline, and `--value` input forms.
