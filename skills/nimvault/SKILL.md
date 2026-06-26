---
name: nimvault
description: >
  Use when sealing/unsealing GPG vault blobs, checking nimvault status, adding
  secrets to .vault/, or when the user mentions nimvault, opaque-blob vault,
  or agent-safe secret storage in a git repo.
---

# nimvault (via MCP)

Prefer **MCP tools** (`nimvault_list`, `nimvault_status`, `nimvault_scan`, …) over shelling `nimvault` directly when the nimvault MCP server is connected.

## Critical rules

Call **nimvault_doctor** first if the user just installed the plugin or tools error with CLI-not-found.

1. **Always set `repo_path`** to the **git repository root** that contains `.vault/` (nimvault is CWD-sensitive). Examples: `~/.local/share/chezmoi`, a project checkout with `.vault/config`.
2. **Mutating tools are blocked** unless the user set `NIMVAULT_MCP_ALLOW_MUTATE=1` in MCP env (add, add_dir, remove, seal, unseal).
3. **Never commit plaintext secrets.** After `nimvault_add`, run `nimvault_seal` and `git add .vault/` only.
4. For fast status, need **nimvault CLI ≥ 0.4.1** and one successful `seal` (manifest v4 `contentHash`).

## Typical flows

**Check health**
- `nimvault_version` then `nimvault_status` with `repo_path`

**Add a secret file**
- Confirm mutate env is enabled (or tell the user to set it)
- `nimvault_add` path + repo_path
- `nimvault_seal` repo_path
- Stage `.vault/*.gpg` and `.vault/manifest.gpg` (not the plaintext)

**Scan for leaks**
- `nimvault_scan` on repo_path (or a subdirectory via `path`)

## Install (if tools missing)

```bash
nimble install nimvault          # CLI
cargo install --git https://github.com/HaoZeke/nimvault-mcp
# Grok: /plugins → install HaoZeke/nimvault-mcp, or:
grok plugin install https://github.com/HaoZeke/nimvault-mcp.git --trust
```

CLI docs: https://nimvault.rgoswami.me
