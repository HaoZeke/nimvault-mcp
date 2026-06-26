# Changelog

## 0.1.5

- Full CLI parity (`mv`), repo walk-up discovery, `nimvault_resolve_repo`
- `NIMVAULT_MCP_READ_ONLY` hard lock + optional audit log
- `docs/SURVEY.md` landscape vs Vault/1Password/sops MCP
- Never exposes vaulted file bodies to the model

## 0.1.3

- GitHub Release multi-arch assets published; doctor points at /releases/latest
- Assert install paths (curl tarball) in doctor output

## 0.1.2

- `nimvault_doctor` tool + `nimvault-mcp doctor` CLI for install/readiness guidance
- Rich MCP `ServerInfo` instructions for agents without prior context
- Errors for missing CLI append install steps (CLI + MCP + Grok plugin + releases URL)
- `NIMVAULT_DEFAULT_REPO`, client-specific `setup --client=…`
- GitHub Actions: CI + tag `v*` multi-arch Release assets (binstall/npm install)

## 0.1.1

- Fast status notes for nimvault CLI 0.4.1+

## 0.1.0

- Initial MCP tools and Grok plugin manifest
