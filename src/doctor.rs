//! Onboarding / readiness checks shown to agents and humans.

use crate::constants::{
    default_recipient_env, default_repo_env, mutate_enabled, nimvault_bin_env, VERSION,
};

/// Long-form instructions for MCP `ServerInfo` (always visible to the model).
pub fn server_instructions() -> String {
    let mut s = String::from(
        "nimvault-mcp talks to the local `nimvault` CLI (GPG opaque-blob vault). \
         It does NOT store secrets in the cloud.\n\n\
         **First-time setup (if tools fail or CLI is missing):**\n\
         1. Install CLI: `nimble install nimvault` (docs: https://nimvault.rgoswami.me)\n\
         2. Install this MCP binary: `cargo install --git https://github.com/HaoZeke/nimvault-mcp` \
            OR `cargo binstall nimvault-mcp` OR download a GitHub Release for your OS, \
            OR `npx @haozeke/nimvault-mcp` after a release publishes npm assets.\n\
         3. Configure a vault in a git repo: `mkdir -p .vault && echo 'recipient = YOUR_GPG_KEY_ID' > .vault/config`\n\
         4. Call tool `nimvault_doctor` for a live checklist.\n\n\
         **Every tool call:** pass `repo_path` = git root that owns `.vault/` \
         (or set env `NIMVAULT_DEFAULT_REPO`). nimvault is CWD-sensitive.\n\n\
         **Safety:** mutating tools (add/add_dir/remove/seal/unseal) are BLOCKED unless \
         `NIMVAULT_MCP_ALLOW_MUTATE=1`. Prefer list/status/scan. Never commit plaintext secrets; \
         only commit `.vault/*.gpg` after seal.\n\n\
         **Performance:** nimvault CLI >= 0.4.1 + one `seal` stores content hashes so status is fast.\n\n\
         **Clients:** Grok `grok plugin install https://github.com/HaoZeke/nimvault-mcp.git --trust`; \
         Claude `claude mcp add nimvault -- nimvault-mcp`; Codex `codex mcp add nimvault -- nimvault-mcp`. \
         Run `nimvault-mcp setup` for full guides.",
    );
    s.push_str(&format!("\n\nMCP version: {VERSION}"));
    s
}

/// Human-readable install block appended to CLI-missing errors.
pub fn install_help_block() -> &'static str {
    "\n\n--- install help ---\n\
     CLI (required):\n\
       nimble install nimvault\n\
       # or: cargo install --git https://github.com/HaoZeke/nimvault  (if published that way)\n\
       Docs: https://nimvault.rgoswami.me\n\
     MCP server:\n\
       cargo install --git https://github.com/HaoZeke/nimvault-mcp\n\
       cargo binstall nimvault-mcp   # after GitHub Releases exist for this version\n\
       # releases: https://github.com/HaoZeke/nimvault-mcp/releases\n\
     Grok plugin (bundles skill + .mcp.json):\n\
       grok plugin install https://github.com/HaoZeke/nimvault-mcp.git --trust\n\
     Then re-run nimvault_doctor or nimvault_version."
}

pub fn format_doctor_report(cli_ok: bool, cli_detail: &str, gpg_hint: &str) -> String {
    let mut lines = vec![
        format!("nimvault-mcp {VERSION} — doctor"),
        String::new(),
        format!(
            "mutate tools: {}",
            if mutate_enabled() {
                "ENABLED (NIMVAULT_MCP_ALLOW_MUTATE)"
            } else {
                "disabled (set NIMVAULT_MCP_ALLOW_MUTATE=1 to allow add/seal/unseal)"
            }
        ),
    ];
    if let Some(r) = default_recipient_env() {
        lines.push(format!("NIMVAULT_GPG_RECIPIENT: set ({r})"));
    } else {
        lines.push(
            "NIMVAULT_GPG_RECIPIENT: unset (ok if each repo has .vault/config recipient = …)"
                .into(),
        );
    }
    if let Some(r) = default_repo_env() {
        lines.push(format!("NIMVAULT_DEFAULT_REPO: {r}"));
    } else {
        lines.push(
            "NIMVAULT_DEFAULT_REPO: unset (pass repo_path on every tool, or set this env)".into(),
        );
    }
    if let Some(b) = nimvault_bin_env() {
        lines.push(format!("NIMVAULT_BIN: {b}"));
    }
    lines.push(String::new());
    if cli_ok {
        lines.push(format!("nimvault CLI: OK — {cli_detail}"));
    } else {
        lines.push(format!("nimvault CLI: MISSING — {cli_detail}"));
        lines.push(install_help_block().trim_start_matches('\n').into());
    }
    if !gpg_hint.is_empty() {
        lines.push(String::new());
        lines.push(gpg_hint.into());
    }
    lines.push(String::new());
    lines.push("Next: nimvault_list / nimvault_status with repo_path=…/checkout/with/.vault".into());
    lines.join("\n")
}

/// Stderr banner once at MCP process start (not on stdout — keeps JSON-RPC clean).
pub fn emit_startup_stderr() {
    let missing_cli = which::which("nimvault").is_err() && nimvault_bin_env().is_none();
    if missing_cli {
        eprintln!(
            "nimvault-mcp {VERSION}: nimvault CLI not found on PATH.\n\
             Agents: call tool nimvault_doctor for install steps.\n\
             Humans: nimble install nimvault && cargo install --git https://github.com/HaoZeke/nimvault-mcp\n\
             Releases: https://github.com/HaoZeke/nimvault-mcp/releases"
        );
    } else if !mutate_enabled() {
        eprintln!(
            "nimvault-mcp {VERSION}: ready (mutate tools OFF). Set NIMVAULT_MCP_ALLOW_MUTATE=1 to seal/add."
        );
    }
}
