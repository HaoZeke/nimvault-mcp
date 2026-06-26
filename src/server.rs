//! MCP tool router over the nimvault CLI.

use rmcp::ServerHandler;
use rmcp::{
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::*,
    tool, tool_handler, tool_router,
};

use crate::cli::run_nimvault;
use crate::constants::{default_recipient_env, mutate_enabled};
use crate::doctor::{format_doctor_report, install_help_block, server_instructions};
use crate::tool_args::*;

#[derive(Clone)]
pub struct Server {
    tool_router: ToolRouter<Self>,
}

fn trunc(s: &str, max: usize) -> String {
    let t = s.trim();
    if t.len() <= max {
        t.to_string()
    } else {
        format!("{}\n… truncated ({} bytes total)", &t[..max], t.len())
    }
}

fn blocked_mutate() -> String {
    "BLOCKED: set NIMVAULT_MCP_ALLOW_MUTATE=1 in the MCP server env (Grok plugin / Claude mcpServers / Codex mcp_servers) to enable add/remove/seal/unseal (writes `.vault/` and may decrypt secrets). list/status/scan remain available. Call nimvault_doctor for a full checklist.".into()
}

fn push_recipient(args: &mut Vec<String>, recipient: &Option<String>) {
    let r = recipient
        .as_ref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(default_recipient_env);
    if let Some(r) = r {
        args.push("--recipient".into());
        args.push(r);
    }
}

#[tool_router]
impl Server {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        name = "nimvault_version",
        description = "Return nimvault-mcp and nimvault CLI versions.",
        annotations(title = "nimvault version", read_only_hint = true, idempotent_hint = true)
    )]
    async fn nimvault_version(&self, Parameters(_a): Parameters<EmptyArgs>) -> String {
        let mcp = crate::constants::version_output();
        match run_nimvault(&["--version".into()], &None).await {
            Ok(o) => format!("{mcp}\n{}", o.display()),
            Err(e) => match run_nimvault(&["--help".into()], &None).await {
                Ok(o2) => format!(
                    "{mcp}\n(no --version; help snippet)\n{}",
                    trunc(&o2.display(), 500)
                ),
                Err(_) => format!("{mcp}\nERROR: {e}{}", &install_help_block()),
            },
        }
    }

    #[tool(
        name = "nimvault_doctor",
        description = "Diagnose nimvault CLI / MCP readiness and print install steps for missing pieces. Call this first when setup is unclear or tools return CLI-not-found errors.",
        annotations(title = "nimvault doctor", read_only_hint = true, idempotent_hint = true)
    )]
    async fn nimvault_doctor(&self, Parameters(_a): Parameters<EmptyArgs>) -> String {
        let (cli_ok, detail) = match run_nimvault(&["--help".into()], &None).await {
            Ok(o) => {
                let d = o.display();
                let snippet = d.lines().take(3).collect::<Vec<_>>().join(" | ");
                (
                    o.ok
                        || d.to_ascii_lowercase().contains("nimvault")
                        || d.contains("SUBCMD"),
                    snippet,
                )
            }
            Err(e) => (false, e),
        };
        let gpg = if cli_ok {
            "GPG: ensure your agent can unlock the key in .vault/config (recipient). \
status/seal/unseal need an unlocked agent on this host."
        } else {
            ""
        };
        format_doctor_report(cli_ok, &detail, gpg)
    }


    #[tool(
        name = "nimvault_list",
        description = "List all vault entries (opaque id + target path). Requires repo_path with `.vault/`.",
        annotations(title = "nimvault list", read_only_hint = true, idempotent_hint = true)
    )]
    async fn nimvault_list(&self, Parameters(a): Parameters<RepoArgs>) -> String {
        match run_nimvault(&["list".into()], &a.repo_path).await {
            Ok(o) if o.ok => trunc(&o.display(), 48_000),
            Ok(o) => format!("FAILED\n{}", trunc(&o.display(), 8_000)),
            Err(e) => {
                let mut msg = format!("ERROR: {e}");
                if msg.contains("not found") || msg.contains("NIMVAULT_BIN") {
                    msg.push_str(&install_help_block());
                }
                msg
            },
        }
    }

    #[tool(
        name = "nimvault_status",
        description = "Show sync status of vault entries vs plaintext on disk ([in-sync]/[modified]/[missing]).",
        annotations(title = "nimvault status", read_only_hint = true, idempotent_hint = true)
    )]
    async fn nimvault_status(&self, Parameters(a): Parameters<RepoArgs>) -> String {
        match run_nimvault(&["status".into()], &a.repo_path).await {
            Ok(o) if o.ok => trunc(&o.display(), 48_000),
            Ok(o) => format!("FAILED (GPG/agent?)\n{}", trunc(&o.display(), 8_000)),
            Err(e) => {
                let mut msg = format!("ERROR: {e}");
                if msg.contains("not found") || msg.contains("NIMVAULT_BIN") {
                    msg.push_str(&install_help_block());
                }
                msg
            },
        }
    }

    #[tool(
        name = "nimvault_scan",
        description = "Scan for unvaulted secret-like files (`nimvault scan`). Read-only.",
        annotations(title = "nimvault scan", read_only_hint = true, idempotent_hint = true)
    )]
    async fn nimvault_scan(&self, Parameters(a): Parameters<ScanArgs>) -> String {
        let mut args = vec!["scan".into()];
        if let Some(p) = a.path.filter(|s| !s.is_empty()) {
            args.push(p);
        }
        match run_nimvault(&args, &a.repo_path).await {
            Ok(o) => trunc(&o.display(), 48_000),
            Err(e) => {
                let mut msg = format!("ERROR: {e}");
                if msg.contains("not found") || msg.contains("NIMVAULT_BIN") {
                    msg.push_str(&install_help_block());
                }
                msg
            },
        }
    }

    #[tool(
        name = "nimvault_add",
        description = "Add a file to the vault (`nimvault add`). Requires NIMVAULT_MCP_ALLOW_MUTATE=1.",
        annotations(title = "nimvault add", read_only_hint = false, destructive_hint = false)
    )]
    async fn nimvault_add(&self, Parameters(a): Parameters<PathRepoArgs>) -> String {
        if !mutate_enabled() {
            return blocked_mutate();
        }
        let mut args = vec!["add".into(), a.path.clone()];
        push_recipient(&mut args, &a.recipient);
        if a.no_gitignore.unwrap_or(false) {
            args.push("--no-gitignore".into());
        }
        match run_nimvault(&args, &a.repo_path).await {
            Ok(o) if o.ok => o.display(),
            Ok(o) => format!("FAILED\n{}", o.display()),
            Err(e) => {
                let mut msg = format!("ERROR: {e}");
                if msg.contains("not found") || msg.contains("NIMVAULT_BIN") {
                    msg.push_str(&install_help_block());
                }
                msg
            },
        }
    }

    #[tool(
        name = "nimvault_add_dir",
        description = "Add a directory recursively (`nimvault add-dir`). Requires NIMVAULT_MCP_ALLOW_MUTATE=1.",
        annotations(title = "nimvault add-dir", read_only_hint = false, destructive_hint = false)
    )]
    async fn nimvault_add_dir(&self, Parameters(a): Parameters<PathRepoArgs>) -> String {
        if !mutate_enabled() {
            return blocked_mutate();
        }
        let mut args = vec!["add-dir".into(), a.path.clone()];
        push_recipient(&mut args, &a.recipient);
        if a.no_gitignore.unwrap_or(false) {
            args.push("--no-gitignore".into());
        }
        match run_nimvault(&args, &a.repo_path).await {
            Ok(o) if o.ok => o.display(),
            Ok(o) => format!("FAILED\n{}", o.display()),
            Err(e) => {
                let mut msg = format!("ERROR: {e}");
                if msg.contains("not found") || msg.contains("NIMVAULT_BIN") {
                    msg.push_str(&install_help_block());
                }
                msg
            },
        }
    }

    #[tool(
        name = "nimvault_remove",
        description = "Remove a path from the vault manifest/blobs (does not delete plaintext). Requires NIMVAULT_MCP_ALLOW_MUTATE=1.",
        annotations(title = "nimvault remove", read_only_hint = false, destructive_hint = true)
    )]
    async fn nimvault_remove(&self, Parameters(a): Parameters<PathRepoArgs>) -> String {
        if !mutate_enabled() {
            return blocked_mutate();
        }
        let mut args = vec!["remove".into(), a.path.clone()];
        push_recipient(&mut args, &a.recipient);
        match run_nimvault(&args, &a.repo_path).await {
            Ok(o) if o.ok => o.display(),
            Ok(o) => format!("FAILED\n{}", o.display()),
            Err(e) => {
                let mut msg = format!("ERROR: {e}");
                if msg.contains("not found") || msg.contains("NIMVAULT_BIN") {
                    msg.push_str(&install_help_block());
                }
                msg
            },
        }
    }

    #[tool(
        name = "nimvault_seal",
        description = "Re-encrypt all vault entries from plaintext sources (`nimvault seal`). Requires NIMVAULT_MCP_ALLOW_MUTATE=1.",
        annotations(title = "nimvault seal", read_only_hint = false, destructive_hint = true)
    )]
    async fn nimvault_seal(&self, Parameters(a): Parameters<SealArgs>) -> String {
        if !mutate_enabled() {
            return blocked_mutate();
        }
        let mut args = vec!["seal".into()];
        push_recipient(&mut args, &a.recipient);
        match run_nimvault(&args, &a.repo_path).await {
            Ok(o) if o.ok => trunc(&o.display(), 32_000),
            Ok(o) => format!("FAILED\n{}", trunc(&o.display(), 8_000)),
            Err(e) => {
                let mut msg = format!("ERROR: {e}");
                if msg.contains("not found") || msg.contains("NIMVAULT_BIN") {
                    msg.push_str(&install_help_block());
                }
                msg
            },
        }
    }

    #[tool(
        name = "nimvault_unseal",
        description = "Decrypt all vault entries to target paths (`nimvault unseal`). Requires NIMVAULT_MCP_ALLOW_MUTATE=1. Needs GPG agent unlock.",
        annotations(title = "nimvault unseal", read_only_hint = false, destructive_hint = true)
    )]
    async fn nimvault_unseal(&self, Parameters(a): Parameters<UnsealArgs>) -> String {
        if !mutate_enabled() {
            return blocked_mutate();
        }
        let mut args = vec!["unseal".into()];
        if a.allow_unsigned.unwrap_or(false) {
            args.push("--allow-unsigned".into());
        }
        push_recipient(&mut args, &a.recipient);
        match run_nimvault(&args, &a.repo_path).await {
            Ok(o) if o.ok => trunc(&o.display(), 32_000),
            Ok(o) => format!("FAILED\n{}", trunc(&o.display(), 8_000)),
            Err(e) => {
                let mut msg = format!("ERROR: {e}");
                if msg.contains("not found") || msg.contains("NIMVAULT_BIN") {
                    msg.push_str(&install_help_block());
                }
                msg
            },
        }
    }
}

#[tool_handler]
impl ServerHandler for Server {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new(
                "nimvault-mcp",
                env!("CARGO_PKG_VERSION"),
            ))
            .with_instructions(server_instructions())
    }
}
