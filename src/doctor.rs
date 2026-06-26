//! Onboarding / readiness checks shown to agents and humans.

use crate::constants::{
    default_recipient_env, default_repo_env, mutate_enabled, nimvault_bin_env, VERSION,
};

const RELEASES: &str = "https://github.com/HaoZeke/nimvault-mcp/releases";
const RELEASES_LATEST: &str = "https://github.com/HaoZeke/nimvault-mcp/releases/latest";
const LINUX_X64_TGZ: &str =
    "https://github.com/HaoZeke/nimvault-mcp/releases/latest/download/nimvault-mcp-x86_64-unknown-linux-gnu.tar.gz";

/// Long-form instructions for MCP `ServerInfo` (always visible to the model).
pub fn server_instructions() -> String {
    format!(
        "nimvault-mcp talks to the local `nimvault` CLI (GPG opaque-blob vault). \
         It does NOT store secrets in the cloud.\n\n\
         **First-time setup (if tools fail or CLI is missing):**\n\
         1. Install CLI: `nimble install nimvault` (docs: https://nimvault.rgoswami.me)\n\
         2. Install this MCP binary (pick one):\n\
            - `cargo install --git https://github.com/HaoZeke/nimvault-mcp`\n\
            - `cargo binstall nimvault-mcp` (pulls GitHub Release assets)\n\
            - Linux x86_64 tarball (published on every tag):\n\
              `curl -fsSL -o /tmp/nv.tgz {LINUX_X64_TGZ} && tar -C ~/.local/bin -xzf /tmp/nv.tgz && chmod +x ~/.local/bin/nimvault-mcp`\n\
            - All platforms: {RELEASES_LATEST}\n\
            - Grok: `grok plugin install https://github.com/HaoZeke/nimvault-mcp.git --trust` (still needs CLI + binary on PATH for fast start)\n\
         3. Configure a vault: `mkdir -p .vault && echo 'recipient = YOUR_GPG_KEY_ID' > .vault/config`\n\
         4. Call tool `nimvault_doctor` for a live checklist.\n\n\
         **Every tool call:** pass `repo_path` = git root that owns `.vault/` \
         (or set env `NIMVAULT_DEFAULT_REPO`). nimvault is CWD-sensitive.\n\n\
         **Safety:** mutating tools (add/add_dir/remove/seal/unseal) are BLOCKED unless \
         `NIMVAULT_MCP_ALLOW_MUTATE=1`. Prefer list/status/scan. Never commit plaintext secrets; \
         only commit `.vault/*.gpg` after seal.\n\n\
         **Performance:** nimvault CLI >= 0.4.1 + one `seal` stores content hashes so status is fast.\n\n\
         **Clients:** Claude `claude mcp add nimvault -- nimvault-mcp`; \
         Codex `codex mcp add nimvault -- nimvault-mcp`; \
         `nimvault-mcp setup` prints full guides.\n\n\
         MCP version: {VERSION}. Releases: {RELEASES}"
    )
}

/// Human-readable install block appended to CLI-missing errors.
pub fn install_help_block() -> String {
    format!(
        "\n\n--- install help ---\n\
         CLI (required):\n\
           nimble install nimvault\n\
           Docs: https://nimvault.rgoswami.me\n\
         MCP server binary (required; already published on GitHub Releases):\n\
           cargo install --git https://github.com/HaoZeke/nimvault-mcp\n\
           cargo binstall nimvault-mcp\n\
           # Linux x86_64 (always available for latest tag):\n\
           curl -fsSL -o /tmp/nv.tgz {LINUX_X64_TGZ}\n\
           tar -C ~/.local/bin -xzf /tmp/nv.tgz && chmod +x ~/.local/bin/nimvault-mcp\n\
           # All OS archives: {RELEASES_LATEST}\n\
         Grok plugin (skill + .mcp.json; still install CLI + binary above):\n\
           grok plugin install https://github.com/HaoZeke/nimvault-mcp.git --trust\n\
         Then re-run nimvault_doctor or nimvault_version."
    )
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
    lines.push(String::new());
    lines.push(format!("MCP binaries (live): {RELEASES_LATEST}"));
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
             Humans: nimble install nimvault\n\
             MCP binary releases (already published): {RELEASES_LATEST}\n\
             Linux x86_64: curl -fsSL -o /tmp/nv.tgz {LINUX_X64_TGZ} && tar -C ~/.local/bin -xzf /tmp/nv.tgz"
        );
    } else if !mutate_enabled() {
        eprintln!(
            "nimvault-mcp {VERSION}: ready (mutate tools OFF). Set NIMVAULT_MCP_ALLOW_MUTATE=1 to seal/add."
        );
    }
}
