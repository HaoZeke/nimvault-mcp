# nimvault-mcp

[![MIT License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

MCP server for [nimvault](https://github.com/HaoZeke/nimvault) — GPG-encrypted
opaque-blob vaults with hidden filenames. Designed like
[ookcite-mcp](https://github.com/TurtleTech-ehf/ookcite-mcp): Rust `rmcp`, `setup`
subcommand, npm wrapper, Grok `plugin.json`.

Requires the `nimvault` CLI on `PATH` (`nimble install nimvault`).

## Quick start

```bash
cargo install --path .
# or: cargo install --git https://github.com/HaoZeke/nimvault-mcp
nimvault-mcp setup
```

Or with npm (after a release ships binaries):

```bash
npx @haozeke/nimvault-mcp setup
```

## Tools

| Tool | Mutates? | Description |
|------|----------|-------------|
| `nimvault_version` | no | MCP + CLI version |
| `nimvault_list` | no | List vault entries |
| `nimvault_status` | no | Sync status vs disk |
| `nimvault_scan` | no | Find unvaulted secrets |
| `nimvault_add` | **yes** | Add file (needs `NIMVAULT_MCP_ALLOW_MUTATE=1`) |
| `nimvault_add_dir` | **yes** | Add directory recursively |
| `nimvault_remove` | **yes** | Drop manifest entry |
| `nimvault_seal` | **yes** | Re-encrypt all entries |
| `nimvault_unseal` | **yes** | Decrypt all entries |

**Always pass `repo_path`** to the git root that owns `.vault/` — nimvault is
CWD-sensitive.

## Configure

```json
{
  "mcpServers": {
    "nimvault": {
      "command": "nimvault-mcp",
      "env": {
        "NIMVAULT_MCP_ALLOW_MUTATE": "0",
        "NIMVAULT_GPG_RECIPIENT": "YOUR_KEY_ID"
      }
    }
  }
}
```

| Variable | Meaning |
|----------|---------|
| `NIMVAULT_BIN` | Path to `nimvault` if not on `PATH` |
| `NIMVAULT_GPG_RECIPIENT` | Default `--recipient` for mutate tools |
| `NIMVAULT_MCP_ALLOW_MUTATE` | `1` / `true` enables add/remove/seal/unseal (default off) |

### Grok Build

```bash
grok plugin install https://github.com/HaoZeke/nimvault-mcp.git
# or: grok plugin install /path/to/nimvault-mcp
```

## Development

```bash
cargo test
cargo build --release
# smoke tools/list (initialize first in real clients)
```

## See also

- CLI docs: <https://nimvault.rgoswami.me>
- Upstream CLI: <https://github.com/HaoZeke/nimvault>
- Design notes (vault): `Software/nimvault/Nimvault_MCP.org` in the obsidian notes vault

## License

MIT
