# Changelog

## Unreleased

### Changes

- Added a local release wrapper for version sync, test gates, tagging, and release workflow verification.
- Documented the local release wrapper in the repo agent guide.

## 0.1.2 - 2026-05-13

### Features

- Added `zot add --collection` and `zot add --tag` for assigning metadata while creating or dry-running items.
- Added `zot delete` with version-checked, `--yes`-guarded Zotero item deletion and dry-run output.
- Added `zot update` for version-checked item PATCH updates of titles, URLs, tags, and collection membership.
- Added `zot attach` for creating imported-file attachment items and uploading local PDFs to Zotero storage.
- Added `zot groups` for listing groups available from a Zotero user profile.
- Added pagination and sorting flags for item, search, and collection list commands, including `--start`, `--all`, `--sort`, and `--direction`.
- Added `zot tags` and `zot items --tag` for listing tags and filtering items by tag.
- Added `zot children` for listing child attachments and notes under an item.
- Added `zot export` for exporting items, collections, and library slices as BibTeX, RIS, CSL JSON, or formatted bibliography output.
- Added `zot items` for listing library, top-level, trashed, and collection items.

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
