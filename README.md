# zot

Small Rust CLI for Zotero.

Current scope:

- Zotero Web API or local desktop API
- user, group, or local user library
- `search`, `collections`, `item`, `open`, `pdf`, `add`, `resolve-user`
- text or JSON output

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
```

With 1Password:

```bash
API_KEY=$(op read 'op://Personal/Zotero API/credential')
zot --username gumadeiras --api-key "$API_KEY" collections --limit 3
zot --username gumadeiras --api-key "$API_KEY" open R7G52L39 --print
zot --username gumadeiras --api-key "$API_KEY" pdf R7G52L39 --print
zot --username gumadeiras --api-key "$API_KEY" add --dry-run url https://example.com
```

With local Zotero desktop:

```bash
zot --local collections --limit 10
zot --local search "transformer" --limit 5
zot --local item R7G52L39
```
