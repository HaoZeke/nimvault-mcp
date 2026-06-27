# Changelog
## 0.3.4

- CLI identity probe: real nimvault --version; doctor no longer dumps cligen help
- nimvault_version reports MCP + CLI path/version + libnimvault

## 0.3.3

- Windows release build: gate UDS `serve` on `cfg(unix)` (stdio MCP works everywhere)
- Completes multi-arch matrix including `x86_64-pc-windows-msvc`

## 0.3.2

- Route every vault op through libnimvault when loaded (CLI fallback only if missing)
- MCP session trap test: seal/add do not spawn nimvault CLI with library present
- Prefer nimvault CLI ≥ 0.4.2 + `nimble buildLib` / `NIMVAULT_LIB` for in-process path
- TRANSPORTS.md Tier D documents full inproc-first MCP surface

## 0.3.1

- Prior 0.3.x line (doctor, multi-arch releases, sticky session, UDS serve)

## 0.1.6

- `docs/ARCHITECTURE.md` MCP SE design; `docs/INTEGRATIONS.md` pass/KeePassXC/Enpass
- `policy` module; server is thin adapter; unit tests on policy
- Doctor reports pass/keepassxc-cli presence (import is file pipeline, not secret egress)


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
