# Grok Build marketplace submission

Official index: https://github.com/xai-org/plugin-marketplace  
This repo is the **plugin source**; the marketplace PR only adds a catalog entry.

## Plugin location (source)

| | |
|--|--|
| **Git** | https://github.com/HaoZeke/nimvault-mcp |
| **Local clone** | `/home/rgoswami/Git/Github/Tools/nimvault-mcp` (this machine) |
| **Binary (dev)** | `~/.local/bin/nimvault-mcp` after `cargo install --path .` |

## Catalog entry (paste into marketplace fork)

Add to `.grok-plugin/marketplace.json` → `plugins` array (pin **full** SHA):

```json
{
  "name": "nimvault",
  "description": "GPG opaque-blob vault via nimvault: list/status/scan; gated add/seal/unseal for agent-safe secrets in git repos. Local CLI only — no cloud.",
  "category": "development",
  "source": {
    "source": "url",
    "url": "https://github.com/HaoZeke/nimvault-mcp.git",
    "sha": "6d7da9d34071c740740b97c620f1cbf042f505da"
  },
  "homepage": "https://github.com/HaoZeke/nimvault-mcp",
  "keywords": ["nimvault", "nimvault mcp", "opaque-blob", "gpg vault"],
  "domains": ["nimvault.rgoswami.me", "github.com/HaoZeke/nimvault"]
}
```

Then:

```bash
python3 scripts/generate-plugin-index.py
python3 scripts/validate-catalog.py
python3 scripts/generate-plugin-index.py --check
# open PR on xai-org/plugin-marketplace
```

## Pre-submit checklist

- [x] Public MIT repo with `plugin.json`, `.mcp.json`, `README.md`, `LICENSE`
- [x] `skills/nimvault/SKILL.md` for agent guidance
- [x] Mutate tools gated (`NIMVAULT_MCP_ALLOW_MUTATE`)
- [x] No postinstall RCE; npm install only fetches GitHub release binaries (optional)
- [x] Keywords brand-scoped (not generic `secrets` alone as CTA bait — kept nimvault-specific)
- [ ] Pin SHA updated on each marketplace release
- [ ] PR to xai-org/plugin-marketplace with CONTRIBUTING template

## Local install (no marketplace)

```bash
grok plugin install https://github.com/HaoZeke/nimvault-mcp.git --trust
# or path:
grok plugin install /home/rgoswami/Git/Github/Tools/nimvault-mcp --trust
```

Requires `nimvault` CLI on PATH (`nimble install nimvault`) and preferably `nimvault-mcp` on PATH for fast startup (launcher falls back to `cargo run --release`).
