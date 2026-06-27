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

#[allow(dead_code)] // public helper for callers that skip sticky session
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
    // Prefer libnimvault for any op that has a symbol; CLI only if None / load miss.
    if let Some((ok, stdout, stderr)) = crate::inproc::try_inproc(args, &workdir) {
        return Ok(NimvaultOutput {
            ok,
            code: Some(if ok { 0 } else { 1 }),
            stdout,
            stderr,
        });
    }
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
    use std::sync::Mutex;

    /// Serialise env mutation across async tests in this module.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[tokio::test]
    async fn help_runs_if_installed() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        // Avoid inheriting NIMVAULT_BIN trap from a sibling test that raced before locking.
        let prev_bin = std::env::var_os("NIMVAULT_BIN");
        std::env::remove_var("NIMVAULT_BIN");
        if which::which("nimvault").is_err() {
            if let Some(v) = prev_bin {
                std::env::set_var("NIMVAULT_BIN", v);
            }
            return;
        }
        let o = run_nimvault_in(&["--help".into()], Path::new(".")).await.expect("run");
        if let Some(v) = prev_bin {
            std::env::set_var("NIMVAULT_BIN", v);
        } else {
            std::env::remove_var("NIMVAULT_BIN");
        }
        let blob = format!("{}{}", o.stdout, o.stderr);
        assert!(
            blob.to_ascii_lowercase().contains("nimvault")
                || blob.contains("SUBCMD")
                || blob.contains("seal"),
            "{blob}"
        );
    }

    /// MCP session path must use libnimvault for seal/add when loaded — prove by
    /// pointing NIMVAULT_BIN at a trap that fails if the CLI is ever spawned.
    #[tokio::test]
    async fn session_mutate_uses_inproc_not_cli_spawn() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        struct RestoreEnv {
            bin: Option<std::ffi::OsString>,
            lib: Option<std::ffi::OsString>,
        }
        impl Drop for RestoreEnv {
            fn drop(&mut self) {
                match &self.bin {
                    Some(v) => std::env::set_var("NIMVAULT_BIN", v),
                    None => std::env::remove_var("NIMVAULT_BIN"),
                }
                match &self.lib {
                    Some(v) => std::env::set_var("NIMVAULT_LIB", v),
                    None => std::env::remove_var("NIMVAULT_LIB"),
                }
            }
        }
        let _restore = RestoreEnv {
            bin: std::env::var_os("NIMVAULT_BIN"),
            lib: std::env::var_os("NIMVAULT_LIB"),
        };

        let so = PathBuf::from("/home/rgoswami/Git/Github/Tools/nimvault/lib/libnimvault.so");
        if !so.is_file() {
            return; // skip without buildLib
        }
        // Prefer explicit path before any OnceLock load in this process.
        std::env::set_var("NIMVAULT_LIB", so.as_os_str());
        assert!(
            crate::inproc::lib_loaded(),
            "libnimvault must load from {}",
            so.display()
        );
        let ver = crate::inproc::lib_version().unwrap_or_default();
        assert!(
            ver.contains("0.4") || ver.contains("lib"),
            "unexpected lib version {ver}"
        );

        // Direct symbol path (no session) must bind seal/add.
        let probe = crate::inproc::try_inproc(
            &["seal".into()],
            Path::new("/nonexistent/nv_probe_repo_xyz"),
        );
        assert!(
            probe.is_some(),
            "try_inproc(seal) must be Some when full ABI lib is loaded (got None => CLI fallback)"
        );

        let scratch = std::env::temp_dir().join(format!(
            "nv_mcp_inproc_ev_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&scratch);
        std::fs::create_dir_all(scratch.join(".vault")).unwrap();
        std::fs::write(
            scratch.join(".vault/config"),
            "recipient = test-no-such-key@invalid\n",
        )
        .unwrap();
        let secret = scratch.join("secret.txt");
        std::fs::write(&secret, "mcp-inproc-evidence\n").unwrap();

        let trap = scratch.join("trap_nimvault.sh");
        std::fs::write(
            &trap,
            "#!/bin/sh\necho TRAP_CLI_SPAWNED >&2\nexit 99\n",
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&trap).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&trap, perms).unwrap();
        }

        std::env::set_var("NIMVAULT_BIN", trap.as_os_str());

        let session = crate::session::Session::new();
        let repo = Some(scratch.to_string_lossy().into_owned());

        // Unknown op falls through to CLI trap — proves trap would fire if used.
        let miss = run_nimvault_session(
            &session,
            &["__not_a_real_op__".into()],
            &repo,
        )
        .await
        .expect("session returns even on trap");
        assert!(
            !miss.ok && miss.stderr.contains("TRAP_CLI_SPAWNED"),
            "trap must run for unknown op; got ok={} stderr={}",
            miss.ok,
            miss.stderr
        );

        // list/status/seal/add must hit inproc (trap never runs).
        for args in [
            vec!["list".into()],
            vec!["status".into()],
            vec!["seal".into()],
            vec![
                "add".into(),
                secret.to_string_lossy().into_owned(),
                "--no-gitignore".into(),
            ],
        ] {
            // Pre-check: inproc must claim the op (MCP session uses same try_inproc).
            let direct = crate::inproc::try_inproc(&args, &scratch);
            assert!(
                direct.is_some(),
                "try_inproc({:?}) returned None — CLI would be used",
                args.first()
            );
            let o = run_nimvault_session(&session, &args, &repo)
                .await
                .expect("session");
            let blob = format!("{}{}", o.stdout, o.stderr);
            assert!(
                !blob.contains("TRAP_CLI_SPAWNED"),
                "op {:?} must not spawn CLI when lib loaded; output={blob}",
                args.first()
            );
        }

        let _ = std::fs::remove_dir_all(&scratch);
    }
}
