//! Invoke the nimvault CLI (no shell).

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use tokio::process::Command;
use tokio::time::timeout;

use crate::constants::{default_repo_env, nimvault_bin_env};
use crate::doctor::install_help_block;

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
            parts.push(format!("stderr:
{}", self.stderr.trim_end()));
        }
        if parts.is_empty() {
            format!(
                "(no output; exit {})",
                self.code.map(|c| c.to_string()).unwrap_or_else(|| "?".into())
            )
        } else {
            parts.join("
")
        }
    }
}

fn resolve_bin() -> Result<PathBuf, String> {
    if let Some(p) = nimvault_bin_env() {
        let pb = PathBuf::from(&p);
        if pb.is_file() {
            return Ok(pb);
        }
        return Err(format!("NIMVAULT_BIN not a file: {p}"));
    }
    which::which("nimvault").map_err(|e| {
        format!(
            "nimvault not found on PATH ({e}). Set NIMVAULT_BIN or install the CLI.{}",
            install_help_block()
        )
    })
}

fn resolve_workdir(repo_path: &Option<String>) -> Result<PathBuf, String> {
    let chosen = repo_path
        .as_ref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(default_repo_env);
    match chosen {
        Some(p) => {
            let path = PathBuf::from(&p);
            if !path.exists() {
                return Err(format!("repo_path does not exist: {p}. Pass the git root that contains .vault/ (or set NIMVAULT_DEFAULT_REPO)."));
            }
            Ok(path)
        }
        None => std::env::current_dir().map_err(|e| format!("Cannot get current dir: {e}")),
    }
}

/// Run `nimvault <args>` with `current_dir` = repo root (nimvault is CWD-sensitive).
pub async fn run_nimvault(args: &[String], repo_path: &Option<String>) -> Result<NimvaultOutput, String> {
    let workdir = resolve_workdir(repo_path)?;
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
        .map_err(|_| format!("nimvault timed out after {DEFAULT_TIMEOUT_SECS}s (GPG may be waiting for a passphrase)"))?
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
        let o = run_nimvault(&["--help".into()], &None).await.expect("run");
        // cligen may use exit 0 with usage on stdout or stderr
        let blob = format!("{}{}", o.stdout, o.stderr);
        assert!(blob.to_ascii_lowercase().contains("nimvault") || blob.contains("SUBCMD") || blob.contains("seal"), "{blob}");
    }
}
