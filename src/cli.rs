//! Invoke the nimvault CLI (no shell) + repo discovery + audit.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use tokio::process::Command;
use tokio::time::timeout;

use crate::constants::{audit_log_path, default_repo_env, nimvault_bin_env};

const DEFAULT_TIMEOUT_SECS: u64 = 600;

#[derive(Debug, Clone)]
pub struct NimvaultOutput {
    pub ok: bool,
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

impl NimvaultOutput {
    pub fn display(&self) -> String {
        let mut parts = Vec::new();
        if !self.stdout.is_empty() {
            parts.push(self.stdout.trim_end().to_string());
        }
        if !self.stderr.is_empty() {
            parts.push(format!("stderr:\n{}", self.stderr.trim_end()));
        }
        if parts.is_empty() {
            format!(
                "(no output; exit {})",
                self.code
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "?".into())
            )
        } else {
            parts.join("\n")
        }
    }
}

fn resolve_bin() -> Result<PathBuf, String> {
    if let Some(p) = nimvault_bin_env() {
        let pb = PathBuf::from(&p);
        if pb.is_file() {
            return Ok(pb);
        }
        return Err(format!(
            "NIMVAULT_BIN not a file: {p}{}",
            crate::doctor::install_help_block()
        ));
    }
    which::which("nimvault").map_err(|e| {
        format!(
            "nimvault not found on PATH ({e}). Set NIMVAULT_BIN or install the CLI.{}",
            crate::doctor::install_help_block()
        )
    })
}

/// Prefer explicit path, then NIMVAULT_DEFAULT_REPO, then walk up for `.vault/`.
pub fn resolve_workdir(repo_path: &Option<String>) -> Result<PathBuf, String> {
    let chosen = repo_path
        .as_ref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(default_repo_env);

    if let Some(p) = chosen {
        let path = PathBuf::from(&p);
        if !path.exists() {
            return Err(format!(
                "repo_path does not exist: {p}. Pass the git root that contains .vault/ \
                 (or set NIMVAULT_DEFAULT_REPO)."
            ));
        }
        return Ok(path);
    }

    // Walk up from CWD looking for .vault/config or .vault/manifest.gpg
    let mut dir = std::env::current_dir().map_err(|e| format!("Cannot get current dir: {e}"))?;
    loop {
        let vault = dir.join(".vault");
        if vault.join("config").is_file() || vault.join("manifest.gpg").is_file() {
            return Ok(dir);
        }
        if !dir.pop() {
            break;
        }
    }

    Err(
        "No repo_path and no .vault found walking up from CWD. \
         Pass repo_path=…/git/root or set NIMVAULT_DEFAULT_REPO, or cd into a vaulted repo."
            .into(),
    )
}

pub fn audit(tool: &str, workdir: &Path, extra: &str) {
    let Some(path) = audit_log_path() else {
        return;
    };
    let line = format!(
        "{}\t{}\t{}\t{}\n",
        chrono_lite_now(),
        tool,
        workdir.display(),
        extra.replace('\t', " ").replace('\n', " ")
    );
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&path) {
        let _ = f.write_all(line.as_bytes());
    }
}

fn chrono_lite_now() -> String {
    // Avoid chrono dep: ISO-ish from system time
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{secs}")
}

pub async fn run_nimvault(
    args: &[String],
    repo_path: &Option<String>,
) -> Result<NimvaultOutput, String> {
    let workdir = resolve_workdir(repo_path)?;
    let tool = args.first().map(|s| s.as_str()).unwrap_or("?");
    audit(tool, &workdir, &args.join(" "));
    run_nimvault_in(args, &workdir).await
}

/// Like [`run_nimvault`] but remembers resolved root on `session` for sticky `repo_path`.
pub async fn run_nimvault_session(
    session: &crate::session::Session,
    args: &[String],
    repo_path: &Option<String>,
) -> Result<NimvaultOutput, String> {
    let effective = session.prefer_repo_arg(repo_path);
    let workdir = resolve_workdir(&effective)?;
    session.remember_root(workdir.clone());
    let tool = args.first().map(|s| s.as_str()).unwrap_or("?");
    audit(tool, &workdir, &args.join(" "));
    run_nimvault_in(args, &workdir).await
}

pub async fn run_nimvault_in(args: &[String], dir: &Path) -> Result<NimvaultOutput, String> {
    let bin = resolve_bin()?;
    let mut cmd = Command::new(&bin);
    cmd.args(args);
    cmd.current_dir(dir);
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    cmd.env("GIT_TERMINAL_PROMPT", "0");

    let child = cmd
        .spawn()
        .map_err(|e| format!("failed to spawn {}: {e}", bin.display()))?;
    let out = timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS), child.wait_with_output())
        .await
        .map_err(|_| {
            format!("nimvault timed out after {DEFAULT_TIMEOUT_SECS}s (GPG may be waiting for a passphrase)")
        })?
        .map_err(|e| format!("nimvault wait failed: {e}"))?;

    Ok(NimvaultOutput {
        ok: out.status.success(),
        code: out.status.code(),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn help_runs_if_installed() {
        if which::which("nimvault").is_err() {
            return;
        }
        let o = run_nimvault_in(&["--help".into()], Path::new(".")).await.expect("run");
        let blob = format!("{}{}", o.stdout, o.stderr);
        assert!(
            blob.to_ascii_lowercase().contains("nimvault")
                || blob.contains("SUBCMD")
                || blob.contains("seal"),
            "{blob}"
        );
    }
}
