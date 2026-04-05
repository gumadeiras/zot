# zot

Small Rust CLI for Zotero.

Current scope:

- Zotero Web API only
- user or group libraries
- `search`, `collections`, `item`, `open`, `pdf`, `add`, `resolve-user`
- text or JSON output

## Auth

Set one of:

- `ZOTERO_USER_ID`
- `ZOTERO_USERNAME`
- `ZOTERO_GROUP_ID`

Optional for public libraries, recommended otherwise:

- `ZOTERO_API_KEY`

`ZOTERO_USERNAME` resolves the numeric user id from the public Zotero profile page.
That works without auth.

Private libraries still need an API key.
`zot` does not create that key for you, because Zotero's web login is behind a Cloudflare
challenge.

Creating items also requires a write-enabled API key.

## Examples

```bash
zot --user-id 123456 search "attention is all you need"
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
