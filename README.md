# zot

Small Rust CLI for Zotero.

Current scope:

- Zotero Web API only
- user or group libraries
- `search`, `collections`, `item`, `resolve-user`
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

## Examples

```bash
zot --user-id 123456 search "attention is all you need"
zot --username gumadeiras collections
zot item ABCD1234
zot resolve-user gumadeiras
zot --json search "transformer" --limit 5
```
