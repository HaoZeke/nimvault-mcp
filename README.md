# nimvault-mcp

[![MIT License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![GitHub](https://img.shields.io/badge/github-HaoZeke%2Fnimvault--mcp-blue)](https://github.com/HaoZeke/nimvault-mcp)

MCP server for [nimvault](https://github.com/HaoZeke/nimvault) — GPG-encrypted
opaque-blob vaults with hidden filenames. Layout mirrors
[ookcite-mcp](https://github.com/TurtleTech-ehf/ookcite-mcp) (Rust `rmcp`, `setup`,
npm wrapper, Grok `plugin.json` + skill).

**Requires** the `nimvault` CLI on `PATH` (`nimble install nimvault`).

## Where is this?

| What | Path / URL |
|------|------------|
| **Source (this repo)** | https://github.com/HaoZeke/nimvault-mcp |
| **Typical local clone** | `~/Git/Github/Tools/nimvault-mcp` |
| **Installed binary** | `~/.local/bin/nimvault-mcp` or `~/.cargo/bin/nimvault-mcp` |
| **Upstream CLI** | https://github.com/HaoZeke/nimvault · docs https://nimvault.rgoswami.me |
| **Grok marketplace packet** | [MARKETPLACE.md](./MARKETPLACE.md) · `marketplace-entry.json` |

Legacy prototypes (superseded for publish): `nimtasks/nimvault_mcp` (Nim),
`nimtasks/nimvault_mcp_rs` (workspace crate).

## Quick start

```bash
# CLI + MCP
nimble install nimvault
cargo install --git https://github.com/HaoZeke/nimvault-mcp
nimvault-mcp setup              # add-mcp + Grok/Claude/Codex guides
nimvault-mcp setup --client=grok
nimvault-mcp setup --client=claude
nimvault-mcp setup --client=codex
```

## Grok Build (plugin)

```bash
grok plugin install https://github.com/HaoZeke/nimvault-mcp.git --trust
# or from a checkout:
grok plugin install /path/to/nimvault-mcp --trust
```

Ships `plugin.json`, `.mcp.json`, `skills/nimvault/SKILL.md`, `commands/`,
`scripts/run-mcp.sh`. Trust the plugin so MCP tools load (Ctrl+L → Plugins).

**Official registry:** submit via [xai-org/plugin-marketplace](https://github.com/xai-org/plugin-marketplace)
using the entry in `MARKETPLACE.md` (pin full commit SHA). Until merged, install
from the git URL above.

Optional `~/.grok/config.toml` overrides:

```toml
[mcp_servers.nimvault.env]
NIMVAULT_DEFAULT_REPO = "/home/you/.local/share/chezmoi"
NIMVAULT_GPG_RECIPIENT = "YOUR_KEY_ID"
NIMVAULT_MCP_ALLOW_MUTATE = "0"
```

## Claude Code / Desktop

```bash
claude mcp add nimvault --env NIMVAULT_MCP_ALLOW_MUTATE=0 -- nimvault-mcp
```

Or merge into `~/.claude.json` / project `.mcp.json` under `mcpServers` (see
`nimvault-mcp setup --client=claude`). Allow `mcp__nimvault__*` in tool permissions
if you use an allowlist.

## Codex CLI

```bash
codex mcp add nimvault \
  --env NIMVAULT_MCP_ALLOW_MUTATE=0 \
  --env NIMVAULT_DEFAULT_REPO="$HOME/.local/share/chezmoi" \
  -- nimvault-mcp
```

## Tools

| Tool | Mutates? | Description |
|------|----------|-------------|
| `nimvault_version` | no | MCP + CLI version |
| `nimvault_list` | no | List vault entries |
| `nimvault_status` | no | Sync status vs disk (fast with CLI ≥0.4.1 + sealed v4 manifest) |
| `nimvault_scan` | no | Find unvaulted secrets |
| `nimvault_add` | **yes** | Add file (`NIMVAULT_MCP_ALLOW_MUTATE=1`) |
| `nimvault_add_dir` | **yes** | Add directory recursively |
| `nimvault_remove` | **yes** | Drop manifest entry |
| `nimvault_seal` | **yes** | Re-encrypt all entries |
| `nimvault_unseal` | **yes** | Decrypt all entries |

Pass **`repo_path`** (git root with `.vault/`), or set **`NIMVAULT_DEFAULT_REPO`**.

| Variable | Meaning |
|----------|---------|
| `NIMVAULT_BIN` | Path to `nimvault` if not on `PATH` |
| `NIMVAULT_GPG_RECIPIENT` | Default `--recipient` for mutate tools |
| `NIMVAULT_DEFAULT_REPO` | Default `repo_path` when omitted |
| `NIMVAULT_MCP_ALLOW_MUTATE` | `1` / `true` enables mutating tools (default **off**) |
| `NIMVAULT_GPG_PARALLEL` | CLI seal/unseal concurrency (default 8) |

## Performance

Use **nimvault ≥ 0.4.1**, run `nimvault seal` once (manifest v4 `contentHash`), then
`status` is local SHA-256 only (~0.5s vs tens of seconds of GPG decrypts).

## Development

```bash
cargo test
cargo build --release
cp target/release/nimvault-mcp ~/.local/bin/
```

## License

MIT

## Releases

Push a version tag to publish multi-arch binaries to GitHub Releases (used by
`cargo binstall` and the npm postinstall downloader):

```bash
# bump Cargo.toml / plugin.json / npm/package.json together
./scripts/set-version.sh 0.1.2
git commit -am "release: 0.1.2"
git tag v0.1.2
git push origin main --tags
```

Optional repo secrets for the same workflow:

| Secret | Effect |
|--------|--------|
| `CARGO_REGISTRY_TOKEN` | `cargo publish` to crates.io |
| `NPM_TOKEN` | `npm publish` `@haozeke/nimvault-mcp` |

Without those secrets, **GitHub Release assets still publish** — enough for
`cargo binstall nimvault-mcp` once the crate is on crates.io, or direct tarball
download. The MCP `nimvault_doctor` tool points users at Releases if the binary
is missing.
