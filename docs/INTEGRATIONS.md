# Integrating pass, KeePassXC, Enpass with nimvault

nimvault stores **whole files** as GPG **opaque blobs** in a **git repo**. Password
managers store **records** (title, user, password, TOTP, attachments). They solve
different problems; integration is a **pipeline**, not a merge of backends.

## Architecture (safe pattern)

```
┌─────────────┐     export/attach      ┌──────────────┐     nimvault add      ┌────────────┐
│ pass /      │  ───────────────────►  │ temp file or │  ─────────────────►  │ .vault/    │
│ KeePassXC / │   (human or gated      │ path on disk │   + seal + git add   │ blobs+     │
│ Enpass      │    import tool)        │              │                      │ manifest   │
└─────────────┘                        └──────────────┘                      └────────────┘
        ▲                                      │
        │         never: MCP returns password fields to the LLM
        └──────────────────────────────────────┘
```

**Rule:** Prefer PM CLIs that write to a **file descriptor or path** the agent
never echoes. MCP must not implement `pass show` → tool result string.

## pass (password-store)

| | |
|--|--|
| **Trust** | Same GPG keyring as nimvault often — good fit |
| **CLI** | `pass`, `pass show -c`, `pass git` |
| **Workflow** | `pass show path/to/entry > /tmp/x` (or `pass show entry \| install -m 600 /secure/path`) → `nimvault add /secure/path` → `nimvault seal` → commit `.vault/` → shred temp |
| **MCP** | Optional future: `nimvault_import_pass` **only** with `NIMVAULT_MCP_ALLOW_IMPORT=1`, writes via `pass show` to a **user path** then `add` (never stdout to model). Default **off**. |
| **Not** | Using pass as nimvault’s blob store (different layout) |

Shared GPG recipient: set `.vault/config` `recipient =` to the same key id as
`PASSWORD_STORE_KEY` / your pass signing key when you want one unlock for both.

## KeePassXC

| | |
|--|--|
| **CLI** | `keepassxc-cli` (show, export, attachment-export) |
| **Workflow** | Unlock DB once (`keepassxc-cli open` / key file / env per your threat model) → `attachment-export` or `show -a Password` **to a file** → `nimvault add` that file if you need **file-shaped** secrets (certs, JSON, SSH keys). For passwords alone, prefer **staying in KeePassXC** and only vaulting **files**. |
| **MCP** | Same gated import pattern; require `KEEPASSXC_DB` + unlock policy documented; never return `show` output as tool text. |
| **Not** | Mounting `.kdbx` as `.vault/` |

## Enpass

| | |
|--|--|
| **CLI** | No first-class FOSS CLI comparable to `pass` / `keepassxc-cli` on most distros |
| **Workflow** | Enpass **export** (encrypted backup or file attachments) → decrypt/export with their app → path on disk → `nimvault add` / `add-dir` |
| **MCP** | No direct integration without a user-supplied export tool; document only unless Enpass adds a stable CLI |
| **Not** | Scraping the Enpass GUI |

## What we implement in-tree

1. **Documented pipelines** (this file) — always.
2. **Doctor checks** — detect `pass` / `keepassxc-cli` on PATH (informational).
3. **Optional gated import tools** (if enabled in build/env) — write PM → tempfile → `nimvault add` without echoing body.

Default MCP binary may include **detection only** until import tools are explicitly enabled, so agents cannot `pass show` via MCP by surprise.

## Hybrid recommendation

- **Human day-to-day passwords / TOTP:** pass or KeePassXC / Enpass.
- **Agent-edited config files in git (Claude settings, tokens-as-files, SSH keys as files):** nimvault.
- **Runtime inject without git:** `pass show` / `keepassxc-cli` in a **wrapper script** around the agent process — not via MCP tool results.
