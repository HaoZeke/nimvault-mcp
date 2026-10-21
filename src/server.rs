//! MCP tool router — thin adapter over `policy` + `cli` runner.
//!
//! See `docs/ARCHITECTURE.md`. No tool returns vaulted file bodies.

use rmcp::ServerHandler;
use rmcp::{
    handler::server::{tool::ToolRouter, wrapper::Json, wrapper::Parameters},
    model::*,
    tool, tool_handler, tool_router,
};

use crate::cli::{resolve_workdir, run_nimvault_session};
use crate::doctor::server_instructions;
use crate::policy::{
    agent_view, enrich_error as err_help, ensure_mutate, push_recipient, require_path_arg, trunc,
};
use crate::report::{
    assemble_doctor, parse_check, parse_status, parse_vault_doctor, parse_who, CheckReport,
    DoctorReport, McpReadiness, StatusReport, VaultDoctor, WhoReport, WriteReport,
};

fn to_agent(s: impl AsRef<str>) -> String {
    agent_view(s.as_ref())
}

async fn mcp_readiness() -> McpReadiness {
    let (cli_ok, cli) = crate::cli::cli_identity().await;
    McpReadiness {
        version: crate::constants::VERSION.into(),
        mutate: crate::constants::mutate_enabled(),
        read_only: crate::constants::read_only_locked(),
        cli_ok,
        cli,
        lib_loaded: crate::inproc::lib_loaded(),
        lib_version: crate::inproc::lib_version(),
        default_repo: crate::constants::default_repo_env(),
        hint: if cli_ok {
            None
        } else {
            Some(crate::doctor::install_help_block())
        },
    }
}

fn repo_string(path: &std::path::Path) -> String {
    path.display().to_string()
}

async fn run_write(
    session: &crate::session::Session,
    command: &str,
    args: &[String],
    repo_path: &Option<String>,
    path: Option<String>,
    force: Option<bool>,
    rekey: Option<bool>,
) -> Json<WriteReport> {
    if let Err(m) = ensure_mutate() {
        return Json(WriteReport::failure(command, "", path, force, rekey, &m));
    }
    let effective = session.prefer_repo_arg(repo_path);
    let repo = match resolve_workdir(&effective) {
        Ok(p) => repo_string(&p),
        Err(e) => {
            return Json(WriteReport::failure(command, "", path, force, rekey, &e));
        }
    };
    match run_nimvault_session(session, args, repo_path).await {
        Ok(o) if o.ok => Json(WriteReport::success(command, &repo, path, force, rekey)),
        Ok(o) => Json(WriteReport::failure(
            command,
            &repo,
            path,
            force,
            rekey,
            &o.display(),
        )),
        Err(e) => Json(WriteReport::failure(command, &repo, path, force, rekey, &e)),
    }
}

fn read_failed_status(err: &str) -> Json<StatusReport> {
    Json(parse_status("", err, false))
}

fn read_failed_who(err: &str) -> Json<WhoReport> {
    Json(parse_who("", err, false))
}

fn read_failed_check(err: &str) -> Json<CheckReport> {
    Json(parse_check("", err, false))
}

use crate::tool_args::*;

#[derive(Clone)]
pub struct Server {
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
    session: crate::session::Session,
}

#[tool_router]
impl Server {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
            session: crate::session::Session::new(),
        }
    }

    #[tool(
        name = "nimvault_version",
        description = "Return nimvault-mcp and nimvault CLI versions.",
        annotations(
            title = "nimvault version",
            read_only_hint = true,
            idempotent_hint = true
        )
    )]
    async fn nimvault_version(&self, Parameters(_a): Parameters<EmptyArgs>) -> String {
        let mcp = crate::constants::version_output();
        let lib = crate::inproc::lib_version()
            .map(|v| format!("libnimvault {v}"))
            .unwrap_or_else(|| "libnimvault (not loaded)".into());
        let (cli_ok, cli) = crate::cli::cli_identity().await;
        if cli_ok {
            to_agent(format!("{mcp}\n{cli}\n{lib}"))
        } else {
            to_agent(format!(
                "{mcp}\nCLI: {cli}\n{lib}{}",
                &crate::doctor::install_help_block()
            ))
        }
    }

    #[tool(
        name = "nimvault_doctor",
        description = "Vault doctor (nimvault doctor) plus MCP readiness. Returns JSON: backend, recipient, gpg, manifest, check, and problems. Does not return file contents.",
        annotations(
            title = "nimvault doctor",
            read_only_hint = true,
            idempotent_hint = true
        )
    )]
    async fn nimvault_doctor(&self, Parameters(a): Parameters<RepoArgs>) -> Json<DoctorReport> {
        let mcp = mcp_readiness().await;
        let effective = self.session.prefer_repo_arg(&a.repo_path);
        let asked = effective.is_some();
        match resolve_workdir(&effective) {
            Ok(dir) => {
                self.session.remember_root(dir.clone());
                let repo = Some(repo_string(&dir));
                match run_nimvault_session(&self.session, &["doctor".into()], &repo).await {
                    Ok(o) => Json(assemble_doctor(
                        mcp,
                        true,
                        parse_vault_doctor(&o.stdout, &o.stderr, o.ok),
                    )),
                    Err(e) => {
                        let mut vault = VaultDoctor::default();
                        vault.ok = false;
                        vault.problems.push(e);
                        Json(assemble_doctor(mcp, true, vault))
                    }
                }
            }
            Err(e) => {
                let mut report = assemble_doctor(mcp, false, VaultDoctor::default());
                if asked {
                    report.ok = false;
                    report.problems.push(e);
                }
                Json(report)
            }
        }
    }

    #[tool(
        name = "nimvault_list",
        description = "List vault entries as opaque id + path (shape only; never contents).",
        annotations(title = "nimvault list", read_only_hint = true, idempotent_hint = true)
    )]
    async fn nimvault_list(&self, Parameters(a): Parameters<RepoArgs>) -> String {
        match run_nimvault_session(&self.session, &["list".into()], &a.repo_path).await {
            Ok(o) if o.ok => to_agent(trunc(&o.display(), 48_000)),
            Ok(o) => to_agent(format!("FAILED\n{}", trunc(&o.display(), 8_000))),
            Err(e) => err_help(e.as_str()),
        }
    }

    #[tool(
        name = "nimvault_status",
        description = "Sync status of vault entries vs plaintext on disk. Returns JSON entries with state in-sync, modified, missing, no-blob, or error. Does not return file contents.",
        annotations(
            title = "nimvault status",
            read_only_hint = true,
            idempotent_hint = true
        )
    )]
    async fn nimvault_status(&self, Parameters(a): Parameters<RepoArgs>) -> Json<StatusReport> {
        match run_nimvault_session(&self.session, &["status".into()], &a.repo_path).await {
            Ok(o) => Json(parse_status(&o.stdout, &o.stderr, o.ok)),
            Err(e) => read_failed_status(&err_help(&e)),
        }
    }

    #[tool(
        name = "nimvault_who",
        description = "Recipients that can open each vault entry (nimvault who). Returns JSON path and recipient ids. Does not return file contents.",
        annotations(title = "nimvault who", read_only_hint = true, idempotent_hint = true)
    )]
    async fn nimvault_who(&self, Parameters(a): Parameters<RepoArgs>) -> Json<WhoReport> {
        match run_nimvault_session(&self.session, &["who".into()], &a.repo_path).await {
            Ok(o) => Json(parse_who(&o.stdout, &o.stderr, o.ok)),
            Err(e) => read_failed_who(&err_help(&e)),
        }
    }

    #[tool(
        name = "nimvault_check",
        description = "Blob-vs-manifest consistency (nimvault check). Decrypts nothing. Returns JSON; ok is false when any entry is inconsistent. Does not return file contents.",
        annotations(
            title = "nimvault check",
            read_only_hint = true,
            idempotent_hint = true
        )
    )]
    async fn nimvault_check(&self, Parameters(a): Parameters<RepoArgs>) -> Json<CheckReport> {
        match run_nimvault_session(&self.session, &["check".into()], &a.repo_path).await {
            Ok(o) => Json(parse_check(&o.stdout, &o.stderr, o.ok)),
            Err(e) => read_failed_check(&err_help(&e)),
        }
    }

    #[tool(
        name = "nimvault_scan",
        description = "Scan for unvaulted secret-like files. Returns path, rule, line, and byte size only — never file contents.",
        annotations(title = "nimvault scan", read_only_hint = true, idempotent_hint = true)
    )]
    async fn nimvault_scan(&self, Parameters(a): Parameters<ScanArgs>) -> String {
        let mut args = vec!["scan".into()];
        if let Some(p) = a.path.filter(|s| !s.is_empty()) {
            match resolve_workdir(&a.repo_path) {
                Ok(root) => {
                    let cand = std::path::Path::new(&p);
                    let joined = if cand.is_absolute() {
                        cand.to_path_buf()
                    } else {
                        root.join(cand)
                    };
                    let inside = joined
                        .canonicalize()
                        .ok()
                        .zip(root.canonicalize().ok())
                        .map(|(c, r)| c.starts_with(r))
                        .unwrap_or(false);
                    if !inside {
                        return to_agent(format!(
                            "path must stay under repo_path ({})",
                            root.display()
                        ));
                    }
                    args.push(joined.to_string_lossy().into_owned());
                }
                Err(e) => return err_help(&e),
            }
        }
        match run_nimvault_session(&self.session, &args, &a.repo_path).await {
            Ok(o) => to_agent(trunc(&o.display(), 48_000)),
            Err(e) => err_help(e.as_str()),
        }
    }

    #[tool(
        name = "nimvault_add",
        description = "Add a file to the vault (nimvault add). path is the filesystem path, not file contents. Requires NIMVAULT_MCP_ALLOW_MUTATE=1. Result is JSON and never includes plaintext.",
        annotations(
            title = "nimvault add",
            read_only_hint = false,
            destructive_hint = false
        )
    )]
    async fn nimvault_add(&self, Parameters(a): Parameters<PathRepoArgs>) -> Json<WriteReport> {
        let path = match require_path_arg(&a.path) {
            Ok(p) => p,
            Err(e) => {
                return Json(WriteReport::failure("add", "", None, None, None, &e));
            }
        };
        let mut args = vec!["add".into(), path.clone()];
        push_recipient(&mut args, &a.recipient);
        if a.no_gitignore.unwrap_or(false) {
            args.push("--no-gitignore".into());
        }
        run_write(
            &self.session,
            "add",
            &args,
            &a.repo_path,
            Some(path),
            None,
            None,
        )
        .await
    }

    #[tool(
        name = "nimvault_add_dir",
        description = "Add a directory recursively (`nimvault add-dir`). Requires NIMVAULT_MCP_ALLOW_MUTATE=1.",
        annotations(
            title = "nimvault add-dir",
            read_only_hint = false,
            destructive_hint = false
        )
    )]
    async fn nimvault_add_dir(&self, Parameters(a): Parameters<PathRepoArgs>) -> String {
        if let Err(m) = ensure_mutate() {
            return to_agent(m);
        }
        let path = match require_path_arg(&a.path) {
            Ok(p) => p,
            Err(e) => return err_help(&e),
        };
        let mut args = vec!["add-dir".into(), path];
        push_recipient(&mut args, &a.recipient);
        if a.no_gitignore.unwrap_or(false) {
            args.push("--no-gitignore".into());
        }
        match run_nimvault_session(&self.session, &args, &a.repo_path).await {
            Ok(o) if o.ok => to_agent(o.display()),
            Ok(o) => to_agent(format!("FAILED\n{}", o.display())),
            Err(e) => err_help(e.as_str()),
        }
    }

    #[tool(
        name = "nimvault_remove",
        description = "Remove a path from the vault manifest/blobs (does not delete plaintext). Requires NIMVAULT_MCP_ALLOW_MUTATE=1.",
        annotations(
            title = "nimvault remove",
            read_only_hint = false,
            destructive_hint = true
        )
    )]
    async fn nimvault_remove(&self, Parameters(a): Parameters<PathRepoArgs>) -> String {
        if let Err(m) = ensure_mutate() {
            return to_agent(m);
        }
        let mut args = vec!["remove".into(), a.path.clone()];
        push_recipient(&mut args, &a.recipient);
        match run_nimvault_session(&self.session, &args, &a.repo_path).await {
            Ok(o) if o.ok => to_agent(o.display()),
            Ok(o) => to_agent(format!("FAILED\n{}", o.display())),
            Err(e) => err_help(e.as_str()),
        }
    }

    #[tool(
        name = "nimvault_seal",
        description = "Re-encrypt vault entries (nimvault seal). force is required: false seals changed plaintext only, true re-encrypts every entry. Requires NIMVAULT_MCP_ALLOW_MUTATE=1. Result is JSON and never includes plaintext.",
        annotations(
            title = "nimvault seal",
            read_only_hint = false,
            destructive_hint = true
        )
    )]
    async fn nimvault_seal(&self, Parameters(a): Parameters<SealArgs>) -> Json<WriteReport> {
        let mut args = vec!["seal".into()];
        if a.force {
            args.push("--force".into());
        }
        push_recipient(&mut args, &a.recipient);
        run_write(
            &self.session,
            "seal",
            &args,
            &a.repo_path,
            None,
            Some(a.force),
            None,
        )
        .await
    }

    #[tool(
        name = "nimvault_rotate",
        description = "Rewrap data keys, or re-encrypt payloads (nimvault rotate). rekey is required: false rewraps keys only, true re-encrypts every payload under a fresh data key. Requires NIMVAULT_MCP_ALLOW_MUTATE=1. Result is JSON and never includes plaintext.",
        annotations(
            title = "nimvault rotate",
            read_only_hint = false,
            destructive_hint = true
        )
    )]
    async fn nimvault_rotate(&self, Parameters(a): Parameters<RotateArgs>) -> Json<WriteReport> {
        let mut args = vec!["rotate".into()];
        if a.rekey {
            args.push("--rekey".into());
        }
        push_recipient(&mut args, &a.recipient);
        run_write(
            &self.session,
            "rotate",
            &args,
            &a.repo_path,
            None,
            None,
            Some(a.rekey),
        )
        .await
    }

    #[tool(
        name = "nimvault_unseal",
        description = "Decrypt all vault entries to target paths (`nimvault unseal`). Requires NIMVAULT_MCP_ALLOW_MUTATE=1. Needs GPG agent unlock.",
        annotations(
            title = "nimvault unseal",
            read_only_hint = false,
            destructive_hint = true
        )
    )]
    async fn nimvault_unseal(&self, Parameters(a): Parameters<UnsealArgs>) -> String {
        if let Err(m) = ensure_mutate() {
            return to_agent(m);
        }
        let mut args = vec!["unseal".into()];
        if a.allow_unsigned.unwrap_or(false) {
            args.push("--allow-unsigned".into());
        }
        push_recipient(&mut args, &a.recipient);
        match run_nimvault_session(&self.session, &args, &a.repo_path).await {
            Ok(o) if o.ok => to_agent(trunc(&o.display(), 32_000)),
            Ok(o) => to_agent(format!("FAILED\n{}", trunc(&o.display(), 8_000))),
            Err(e) => err_help(e.as_str()),
        }
    }

    #[tool(
        name = "nimvault_mv",
        description = "Rename a vault entry target path (`nimvault mv old new`). Requires mutate permission. Does not print file contents.",
        annotations(
            title = "nimvault mv",
            read_only_hint = false,
            destructive_hint = false
        )
    )]
    async fn nimvault_mv(&self, Parameters(a): Parameters<MoveArgs>) -> String {
        if let Err(m) = ensure_mutate() {
            return to_agent(m);
        }
        let mut args = vec!["mv".into(), a.old_path.clone(), a.new_path.clone()];
        push_recipient(&mut args, &a.recipient);
        match run_nimvault_session(&self.session, &args, &a.repo_path).await {
            Ok(o) if o.ok => to_agent(o.display()),
            Ok(o) => to_agent(format!("FAILED\n{}", o.display())),
            Err(e) => err_help(e.as_str()),
        }
    }

    #[tool(
        name = "nimvault_resolve_repo",
        description = "Resolve which directory would be used as repo_path (explicit arg, NIMVAULT_DEFAULT_REPO, or walk-up to .vault). Read-only.",
        annotations(
            title = "resolve vault repo",
            read_only_hint = true,
            idempotent_hint = true
        )
    )]
    async fn nimvault_resolve_repo(&self, Parameters(a): Parameters<RepoArgs>) -> String {
        let effective = self.session.prefer_repo_arg(&a.repo_path);
        match resolve_workdir(&effective) {
            Ok(p) => {
                self.session.remember_root(p.clone());
                to_agent(format!(
                    "repo_path={}\nsticky=yes\nhas_vault_config={}\nhas_manifest={}",
                    p.display(),
                    p.join(".vault/config").is_file(),
                    p.join(".vault/manifest.gpg").is_file()
                ))
            }
            Err(e) => err_help(e.as_str()),
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
