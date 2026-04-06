# zot

Small Rust CLI for Zotero.

Current scope:

- Zotero Web API or local desktop API
- user, group, or local user library
- `search`, `collections`, `item`, `open`, `pdf`, `add`, `resolve-user`
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

Creating items also requires a write-enabled API key.

## Examples

```bash
zot --user-id 123456 search "attention is all you need"
zot --local search "attention"
zot --local item ABCD1234
zot --username gumadeiras collections
zot item ABCD1234
zot open ABCD1234
zot pdf ABCD1234 --print
zot resolve-user gumadeiras
zot --json search "transformer" --limit 5
zot add --dry-run doi 10.1038/s41467-025-66107-x
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
zot --username gumadeiras --api-key "$API_KEY" add --dry-run url https://example.com
zot --json --username gumadeiras --api-key "$API_KEY" item R7G52L39
```

With local Zotero desktop:

```bash
zot --local collections --limit 10
zot --local search "transformer" --limit 5
zot --local item R7G52L39
```

## JSON I/O

`--json` emits structured JSON for all commands.

`zot add json` accepts either:

- a single Zotero item object
- a single-item JSON array
- inline JSON with `--value` or as the positional `json` argument

From stdin:

```bash
cat item.json | zot add --dry-run json
cat item.json | zot --json add --dry-run json
```

From a file:

```bash
zot add --dry-run json item.json
zot --json add json item.json
```

Inline:

```bash
zot add --dry-run json '{"itemType":"webpage","title":"Example","url":"https://example.com"}'
zot add --dry-run json --value '{"itemType":"webpage","title":"Example","url":"https://example.com"}'
zot --json add --dry-run json --value '{"itemType":"webpage","title":"Example","url":"https://example.com"}'
```
