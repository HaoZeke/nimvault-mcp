# nimvault-mcp software engineering design

Target: **ookcite-mcp–class** local MCP binary — clear module boundaries, policy
in one place, CLI behind a narrow port, no secret payloads in tool results.

## Layering

```
main ──► setup | doctor(cli) | MCP stdio
              │
              ▼
           server   (#[tool_router] only: map args → policy → port → response text)
              │
     ┌────────┼────────┐
     ▼        ▼        ▼
  policy   runner    doctor
  (mutate,  (nimvault  (readiness
   read_only, CLI spawn,  copy)
   audit)   repo resolve)
     │
     ▼
  constants / tool_args  (env + schemars DTOs)
```

| Module | Responsibility | Must not |
|--------|----------------|----------|
| `main` | argv (`--version`, `setup`, `doctor`), start stdio | Tool business logic |
| `setup` | Client install guides / add-mcp | Spawn nimvault |
| `server` | MCP tool surface only | Long policy strings duplicated ad hoc |
| `policy` | Mutate/read-only gates, recipient merge, error enrichment | I/O |
| `runner` | Resolve workdir, spawn `nimvault`, audit line | MCP types |
| `doctor` | Human/agent install + readiness text | Success-path vault ops |
| `tool_args` | Serde/schemars inputs | Behavior |
| `constants` | VERSION + env knobs | Process spawn |

## Design principles (MCP SE, not product category)

1. **Port over SDK** — talk to the **nimvault CLI** as the system of record (one crypto implementation). No second GPG stack in Rust unless we extract a shared library later.
2. **Policy before spawn** — every mutating tool checks `policy::mutate_allowed()`; read-only lock wins.
3. **No secret egress** — tools return CLI **metadata/status** only; never `cat` vaulted plaintext into the model context.
4. **CWD is hostile** — `runner::resolve_workdir` is explicit → `NIMVAULT_DEFAULT_REPO` → walk-up `.vault`; never assume agent CWD is the vault root without discovery.
5. **Fail loud with install path** — CLI missing / no repo → structured help (doctor block), not a one-line errno.
6. **Observability without leakage** — optional append-only audit: timestamp, tool, workdir, argv summary; never file bodies.
7. **Client adapters are data** — `.mcp.json`, `plugin.json`, `skills/`, `setup` output; binary stays client-agnostic.
8. **Test the pure edges** — policy + workdir resolution unit-tested; CLI integration behind `which nimvault`.

## Comparison to ookcite-mcp

| ookcite-mcp | nimvault-mcp |
|-------------|--------------|
| `endpoints` registry + OpenAPI contract tests | `runner` is the port; CLI is the contract (cligen surface) |
| HTTP client + retries | Local process + timeout (GPG may block on agent) |
| API key in env | GPG agent + optional recipient env |
| No mutate gate (API is multi-tenant) | **Mutate gate + read-only lock** (local secrets) |

## Extension points (integrations)

Bridges to **pass** / **KeePassXC** / **Enpass** belong as **import pipelines** (materialize a file → `nimvault add` → `seal`), not as alternate crypto backends. See `docs/INTEGRATIONS.md`. Keep bridges **out of the default tool list** unless gated (`NIMVAULT_MCP_ALLOW_IMPORT=1`) so agents cannot pull PM entries into context by accident.

## Non-goals

- Replacing Vault/1Password dynamic secrets
- Returning decrypted file contents via MCP
- Shelling out with interpolated secrets (always argv arrays / `Command`)
