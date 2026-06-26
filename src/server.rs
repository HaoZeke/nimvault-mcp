//! MCP tool router — thin adapter over `policy` + `cli` runner.
//!
//! See `docs/ARCHITECTURE.md`. No tool returns vaulted file bodies.

use rmcp::ServerHandler;
use rmcp::{
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::*,
    tool, tool_handler, tool_router,
};

use crate::cli::run_nimvault;
use crate::doctor::{format_doctor_report, server_instructions};
use crate::policy::{
    blocked_mutate_message as blocked_mutate, enrich_error as err_help, ensure_mutate,
    push_recipient, trunc,
};
use crate::tool_args::*;

#[derive(Clone)]
pub struct Server {
    tool_router: ToolRouter<Self>,
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
                Err(_) => format!("{mcp}\nERROR: {e}{}", &crate::doctor::install_help_block()),
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
            Err(e) => err_help(e.as_str()),
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
            Err(e) => err_help(e.as_str()),
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
            Err(e) => err_help(e.as_str()),
        }
    }

    #[tool(
        name = "nimvault_add",
        description = "Add a file to the vault (`nimvault add`). Requires NIMVAULT_MCP_ALLOW_MUTATE=1.",
        annotations(title = "nimvault add", read_only_hint = false, destructive_hint = false)
    )]
    async fn nimvault_add(&self, Parameters(a): Parameters<PathRepoArgs>) -> String {
        if let Err(m) = ensure_mutate() {
            return m;
        }
        let mut args = vec!["add".into(), a.path.clone()];
        push_recipient(&mut args, &a.recipient);
        if a.no_gitignore.unwrap_or(false) {
            args.push("--no-gitignore".into());
        }
        match run_nimvault(&args, &a.repo_path).await {
            Ok(o) if o.ok => o.display(),
            Ok(o) => format!("FAILED\n{}", o.display()),
            Err(e) => err_help(e.as_str()),
        }
    }

    #[tool(
        name = "nimvault_add_dir",
        description = "Add a directory recursively (`nimvault add-dir`). Requires NIMVAULT_MCP_ALLOW_MUTATE=1.",
        annotations(title = "nimvault add-dir", read_only_hint = false, destructive_hint = false)
    )]
    async fn nimvault_add_dir(&self, Parameters(a): Parameters<PathRepoArgs>) -> String {
        if let Err(m) = ensure_mutate() {
            return m;
        }
        let mut args = vec!["add-dir".into(), a.path.clone()];
        push_recipient(&mut args, &a.recipient);
        if a.no_gitignore.unwrap_or(false) {
            args.push("--no-gitignore".into());
        }
        match run_nimvault(&args, &a.repo_path).await {
            Ok(o) if o.ok => o.display(),
            Ok(o) => format!("FAILED\n{}", o.display()),
            Err(e) => err_help(e.as_str()),
        }
    }

    #[tool(
        name = "nimvault_remove",
        description = "Remove a path from the vault manifest/blobs (does not delete plaintext). Requires NIMVAULT_MCP_ALLOW_MUTATE=1.",
        annotations(title = "nimvault remove", read_only_hint = false, destructive_hint = true)
    )]
    async fn nimvault_remove(&self, Parameters(a): Parameters<PathRepoArgs>) -> String {
        if let Err(m) = ensure_mutate() {
            return m;
        }
        let mut args = vec!["remove".into(), a.path.clone()];
        push_recipient(&mut args, &a.recipient);
        match run_nimvault(&args, &a.repo_path).await {
            Ok(o) if o.ok => o.display(),
            Ok(o) => format!("FAILED\n{}", o.display()),
            Err(e) => err_help(e.as_str()),
        }
    }

    #[tool(
        name = "nimvault_seal",
        description = "Re-encrypt all vault entries from plaintext sources (`nimvault seal`). Requires NIMVAULT_MCP_ALLOW_MUTATE=1.",
        annotations(title = "nimvault seal", read_only_hint = false, destructive_hint = true)
    )]
    async fn nimvault_seal(&self, Parameters(a): Parameters<SealArgs>) -> String {
        if let Err(m) = ensure_mutate() {
            return m;
        }
        let mut args = vec!["seal".into()];
        push_recipient(&mut args, &a.recipient);
        match run_nimvault(&args, &a.repo_path).await {
            Ok(o) if o.ok => trunc(&o.display(), 32_000),
            Ok(o) => format!("FAILED\n{}", trunc(&o.display(), 8_000)),
            Err(e) => err_help(e.as_str()),
        }
    }

    #[tool(
        name = "nimvault_unseal",
        description = "Decrypt all vault entries to target paths (`nimvault unseal`). Requires NIMVAULT_MCP_ALLOW_MUTATE=1. Needs GPG agent unlock.",
        annotations(title = "nimvault unseal", read_only_hint = false, destructive_hint = true)
    )]
    async fn nimvault_unseal(&self, Parameters(a): Parameters<UnsealArgs>) -> String {
        if let Err(m) = ensure_mutate() {
            return m;
        }
        let mut args = vec!["unseal".into()];
        if a.allow_unsigned.unwrap_or(false) {
            args.push("--allow-unsigned".into());
        }
        push_recipient(&mut args, &a.recipient);
        match run_nimvault(&args, &a.repo_path).await {
            Ok(o) if o.ok => trunc(&o.display(), 32_000),
            Ok(o) => format!("FAILED\n{}", trunc(&o.display(), 8_000)),
            Err(e) => err_help(e.as_str()),
        }
    }

    #[tool(
        name = "nimvault_mv",
        description = "Rename a vault entry target path (`nimvault mv old new`). Requires mutate permission. Does not print file contents.",
        annotations(title = "nimvault mv", read_only_hint = false, destructive_hint = false)
    )]
    async fn nimvault_mv(&self, Parameters(a): Parameters<MoveArgs>) -> String {
        if let Err(m) = ensure_mutate() {
            return m;
        }
        let mut args = vec!["mv".into(), a.old_path.clone(), a.new_path.clone()];
        push_recipient(&mut args, &a.recipient);
        match run_nimvault(&args, &a.repo_path).await {
            Ok(o) if o.ok => o.display(),
            Ok(o) => format!("FAILED
{}", o.display()),
            Err(e) => err_help(e.as_str()),
        }
    }

    #[tool(
        name = "nimvault_resolve_repo",
        description = "Resolve which directory would be used as repo_path (explicit arg, NIMVAULT_DEFAULT_REPO, or walk-up to .vault). Read-only.",
        annotations(title = "resolve vault repo", read_only_hint = true, idempotent_hint = true)
    )]
    async fn nimvault_resolve_repo(&self, Parameters(a): Parameters<RepoArgs>) -> String {
        match crate::cli::resolve_workdir(&a.repo_path) {
            Ok(p) => format!("repo_path={}
.has_vault_config={}
.has_manifest={}",
                p.display(),
                p.join(".vault/config").is_file(),
                p.join(".vault/manifest.gpg").is_file()),
            Err(e) => format!("ERROR: {e}"),
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
