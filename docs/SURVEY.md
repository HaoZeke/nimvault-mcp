# Landscape survey: secrets + MCP (2026) vs nimvault-mcp

## Categories (not competitors for the same niche)

| Class | Examples | What they optimize | Gap vs nimvault |
|-------|----------|--------------------|-----------------|
| **Cloud / enterprise vault MCP** | HashiCorp Vault MCP, Infisical, AWS SM | Dynamic creds, RBAC, audit, multi-tenant | Network service; secrets often **readable by the model** if tools return values |
| **Password manager MCP** | 1Password MCP, Bitwarden / Agent Access | Human + machine secrets, biometrics, `op run` injection | SaaS/self-host PM; not **git-native opaque filenames** |
| **Git-native config encryption** | sops+age, sops+GPG, git-crypt | Encrypt values in YAML/JSON committed as clear keys | Keys visible in git; not whole-file opaque blobs |
| **Agent secret injection** | `infisical run`, `op run`, `bws run`, Bitwarden Agent SDK | Keep secrets **out of model context** | Complements vaults; not a vault itself |
| **Generic key-value MCP** | Redis, etc. | Fast KV | Not GPG, not git history hygiene |

**MCP security consensus (WorkOS, Infisical, MCP security tutorials):** no secrets in prompts/logs; least privilege; mutate gated; prefer not returning secret **payloads** to the LLM; audit access; runtime env not committed config.

## nimvault’s niche (unique)

1. **Whole files** as GPG ciphertext under **random blob names** (history never holds the path as a secret map in the clear manifest only as encrypted manifest).
2. **Repo-scoped** `.vault/` for agent/dotfile/Claude configs (chezmoi + nimvault pattern).
3. **Local-only** — no cloud API key for the vault itself (GPG key is the trust root).

That is **orthogonal** to Vault/1Password/sops. Hybrid is normal: 1PW for humans, Vault for dynamic infra, **nimvault for “these files must not appear as plaintext paths in git.”**

## Gaps we closed toward “best in niche”

| Gap | Mitigation in nimvault-mcp |
|-----|----------------------------|
| Incomplete CLI surface | `mv` / `rm` aliases; full parity with seal/unseal/add/add-dir/list/status/scan |
| Agents lose `repo_path` | Walk-up `.vault/` discovery + `NIMVAULT_DEFAULT_REPO` |
| Silent fail / no onboarding | `nimvault_doctor`, ServerInfo instructions, install URLs on errors |
| Over-privileged mutate | Default mutate **off**; optional **read-only** hard lock |
| No access trail | Optional **audit log** (paths + tool names, never file contents) |
| Returning secrets to model | **Never** tools that `cat` vaulted plaintext; only status/list/scan metadata |
| Distribution | GitHub Releases multi-arch; doctor points at `/releases/latest` |
| Client packaging | Grok plugin + skill; Claude/Codex setup |

## Still not “perfect” by enterprise vault standards

- No dynamic/ephemeral credentials (by design — GPG long-term keys).
- No multi-user RBAC (OS user + GPG key is the ACL).
- GPG agent unlock is host UX (document headless separately).
- Not a substitute for 1Password/Vault for runtime API keys **inside** running services.

## Verdict

For **git-native, whole-file, opaque-blob, local GPG vaults exposed to coding agents**, nimvault-mcp is the **reference MCP** once CLI ≥0.4.1 and releases are used. It is **not** a general secrets platform; claiming global SOTA against Vault MCP is a category error. Claiming **SOTA in the nimvault niche** is fair after this survey’s gaps are implemented.
