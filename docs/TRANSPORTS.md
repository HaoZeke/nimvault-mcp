# Beyond stdio: transport design for nimvault-mcp

## Why stdio is not the ceiling

Most agent hosts (Grok, Claude Code, Codex) **speak MCP over stdio**: they spawn a
child, JSON-RPC on stdin/stdout, kill the child when the session ends. ookcite-mcp
and nimvault-mcp **0.1.x** follow that because **clients require it**.

Stdio limits for a **local GPG vault** MCP specifically:

| Issue | Effect |
|-------|--------|
| **Process lifetime = session** | Every agent session cold-starts; no warm GPG agent use across sessions unless the agent OS keeps gpg-agent |
| **One client per process** | No sharing a single vault session across tools/windows |
| **Per-tool `nimvault` spawn** | Still true even in a long-lived server unless we add a worker pool / in-process lib |
| **No session state** | `repo_path` repeated; no “bind this connection to this vault” |
| **Stderr only for logs** | Easy to corrupt JSON-RPC if anything prints on stdout |
| **No transport auth** | Fine for single-user local; wrong for multi-user host |

MCP’s other first-class transport is **Streamable HTTP** (POST + optional SSE):
long-lived server, many clients, optional OAuth. That fits **remote** secrets
gateways (Vault MCP), not a laptop GPG keyring unless bound to **localhost** with
careful auth.

## Better than “stdio only” (what we implement)

### Tier A — Unix domain socket daemon (local multi-client)

```
Agent ──stdio──► nimvault-mcp (child, optional thin) 
                      │
                      │  (future: proxy)
                      ▼
              nimvault-mcp serve --socket ~/.cache/nimvault-mcp/mcp.sock
                      │
                      ├── connection 1: MCP over UnixStream (async-rw)
                      └── connection 2: …
                              │
                              ▼
                         nimvault CLI / gpg-agent (host)
```

- **One long-lived process** on the machine; agents that can use sockets attach
  without respawning the server binary’s policy/doctor code path every time.
- **Still no network exposure** if the socket is mode `0600` under `$XDG_RUNTIME_DIR`.
- **stdio remains** for hosts that only know how to spawn children (Grok/Claude today):
  default mode is still `stdio`; `serve` is opt-in.

Command:

```bash
nimvault-mcp serve --socket "${XDG_RUNTIME_DIR:-/tmp}/nimvault-mcp.sock"
# clients that support UDS MCP can connect; otherwise keep using stdio child
```

### Tier B — Session-bound vault root (connection state)

Even on stdio, treat **resolved workdir** as sticky for the process lifetime once
discovered (walk-up / `NIMVAULT_DEFAULT_REPO` / first explicit `repo_path`), so
tools stop re-arguing path. (Stdio session = one process = natural scope.)

### Tier C — Streamable HTTP on loopback (optional, not default)

`127.0.0.1` + token in env for multi-agent on one host **without** UDS support.
Higher footgun (any local process might connect) → require
`NIMVAULT_MCP_HTTP_TOKEN` and document threat model. Prefer UDS on Linux/macOS.

### Tier D — In-process library (**shipped** for MCP vault ops)

`libnimvault.so` C ABI + MCP `inproc::try_inproc` — see section below. CLI spawn
is fallback only when the `.so` or an op symbol is missing.

### Tier E — Human-in-the-loop mutate (MCP elicitation) / brokered IPC

Mutate tools request **client-side confirmation** (MCP elicitation) instead of
only env gates — better SE for “agent wanted seal” accidents. Depends on host
support. Optional later: NNG/ZeroMQ multi-client control plane (non-goal for Tier D).

## What we deliberately keep from the “stdio design”

- **Tool schema + policy + no secret egress** — independent of transport.
- **CLI as port** — one GPG implementation (and shared lib implementation).
- **Default entrypoint = stdio** so Grok/Claude/Codex work without a daemon.

## Verdict

Improving on ookcite-style **stdio-only** for nimvault means **UDS session + Tier D
in-process ops**, not abandoning stdio. Agents still attach via stdio; vault tools
prefer `libnimvault` when loaded.

## Tier D — in-process (complete for MCP surface)

Build: `cd nimvault && nimble buildLib && cp lib/libnimvault.so ~/.local/lib/`
(or set `NIMVAULT_LIB` to the `.so`).

Full C ABI: `nv_list`, `nv_status`, `nv_seal`, `nv_unseal`, `nv_add`, `nv_add_dir`,
`nv_remove`, `nv_mv`, `nv_scan` plus `nv_free` / `nv_last_error` / `nv_version`.

**MCP routing (`run_nimvault_session`):** for **every** op, call `inproc::try_inproc`
first; if it returns `Some`, that result is used (success or library error text).
Only if it returns `None` (no `.so` or unknown op) does the server spawn the
`nimvault` CLI. Mutate tools still pass `NIMVAULT_MCP_ALLOW_MUTATE` / `READ_ONLY`
**before** any library or CLI call. Library paths set `nvQuiet` so scan/seal do
not write progress to the MCP host stdout.
