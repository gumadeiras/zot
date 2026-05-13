# zot

Small Rust CLI for Zotero.

## Install

```bash
brew tap gumadeiras/tap
brew install zot
```

Or build from source:

```bash
cargo install --git https://github.com/gumadeiras/zot.git
```

Current scope:

- Zotero Web API or local desktop API
- user, group, or local user library
- `items`, `tags`, `search`, `collections`, `groups`, `item`, `children`, `open`, `pdf`, `export`, `add`, `attach`, `update`, `delete`, `resolve-user`
- text or JSON output; JSON input for `add`

## Auth

Set one of:

- `ZOTERO_LOCAL=1`
- `ZOTERO_USER_ID`
- `ZOTERO_USERNAME`
- `ZOTERO_GROUP_ID`

Optional for public libraries, recommended otherwise:

- `ZOTERO_API_KEY`

`ZOTERO_USERNAME` resolves the numeric user id from the public Zotero profile page.
That works without auth.

`ZOTERO_LOCAL=1` talks to a running Zotero desktop instance over
`http://localhost:23119/api` and uses the local user library as `users/0`.
That mode is for local reads and does not need an API key.

Private libraries still need an API key.
`zot` does not create that key for you, because Zotero's web login is behind a Cloudflare
challenge.

Creating, attaching, updating, or deleting items also requires a write-enabled API key.

## Examples

```bash
zot --user-id 123456 search "attention is all you need"
zot --user-id 123456 items --limit 20
zot --user-id 123456 items --all --sort title --direction asc
zot --user-id 123456 items --tag neuroscience
zot --user-id 123456 tags --query neuro --qmode starts-with
zot --user-id 123456 items --collection ABCD1234 --top
zot --username gumadeiras groups
zot --local search "attention"
zot --local item ABCD1234
zot --username gumadeiras collections
zot item ABCD1234
zot children ABCD1234
zot export bibtex --item ABCD1234
zot export ris --collection ABCD1234 --output zotero.ris
zot open ABCD1234
zot pdf ABCD1234 --print
zot attach ABCD1234 ./paper.pdf --title "Paper title" --dry-run
zot update ABCD1234 --title "Updated title" --tag reviewed --dry-run
zot delete ABCD1234 --dry-run
zot resolve-user gumadeiras
zot --json search "transformer" --limit 5
zot add --dry-run doi 10.1038/s41467-025-66107-x
zot add --dry-run --collection ABCD1234 --tag neuroscience url https://example.com
echo '{"itemType":"webpage","title":"Example","url":"https://example.com"}' | zot add --dry-run json
zot add --dry-run json '{"itemType":"webpage","title":"Example","url":"https://example.com"}'
zot add --dry-run json --value '{"itemType":"webpage","title":"Example","url":"https://example.com"}'
```

With 1Password:

```bash
API_KEY=$(op read 'op://Personal/Zotero API/credential')
zot --username gumadeiras --api-key "$API_KEY" collections --limit 3
zot --username gumadeiras --api-key "$API_KEY" open R7G52L39 --print
zot --username gumadeiras --api-key "$API_KEY" pdf R7G52L39 --print
zot --username gumadeiras --api-key "$API_KEY" export bibtex --item R7G52L39
zot --username gumadeiras --api-key "$API_KEY" add --dry-run url https://example.com
zot --username gumadeiras --api-key "$API_KEY" attach R7G52L39 ~/Downloads/paper.pdf --title "Paper"
zot --username gumadeiras --api-key "$API_KEY" update R7G52L39 --title "Updated title"
zot --username gumadeiras --api-key "$API_KEY" delete R7G52L39 --yes
zot --json --username gumadeiras --api-key "$API_KEY" item R7G52L39
```

With local Zotero desktop:

```bash
zot --local items --top --limit 10
zot --local tags --limit 10
zot --local collections --limit 10
zot --local search "transformer" --limit 5
zot --local item R7G52L39
zot --local children R7G52L39
```

## JSON I/O

`--json` emits structured JSON for all commands.

`zot add json` accepts either:

- a single Zotero item object
- a single-item JSON array
- inline JSON with `--value` or as the positional `json` argument

Resolution order:

1. `--value`
2. `-` for stdin
3. an existing file path
4. inline JSON in the positional argument

From stdin:

```bash
cat item.json | zot add --dry-run json
cat item.json | zot --json add --dry-run json
```

From a file:

```bash
zot add --dry-run json item.json
zot --json add --dry-run json item.json
```

Inline:

```bash
zot add --dry-run json '{"itemType":"webpage","title":"Example","url":"https://example.com"}'
zot add --dry-run json --value '{"itemType":"webpage","title":"Example","url":"https://example.com"}'
zot --json add --dry-run json --value '{"itemType":"webpage","title":"Example","url":"https://example.com"}'
```

## Release

Tag pushes like `vX.Y.Z` run the release workflow: `cargo test --locked`,
GitHub release, and `gumadeiras/homebrew-tap` update.
