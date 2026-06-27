//! Long-lived local MCP server on a Unix domain socket (better than stdio-only).
//!
//! Each accepted connection runs a full MCP session over the stream (rmcp async-rw).
//! Socket mode 0o600. No network bind.
//!
//! Compiled on Unix only (`main` gates `serve` on non-Unix).

#![cfg(unix)]

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use rmcp::ServiceExt;
use tokio::net::UnixListener;
use tokio::signal;

use crate::server::Server;

pub async fn run_unix_socket(socket: PathBuf) -> Result<()> {
    if socket.exists() {
        let _ = std::fs::remove_file(&socket);
    }
    if let Some(parent) = socket.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    let listener = UnixListener::bind(&socket)
        .with_context(|| format!("bind {}", socket.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&socket, std::fs::Permissions::from_mode(0o600));
    }

    write_pidfile(&socket);

    eprintln!(
        "nimvault-mcp: listening on unix:{} (mode 0600, pid {}). Ctrl-C to stop.\n\
         Default agent entry remains stdio (`nimvault-mcp` with no args).\n\
         Sticky vault root is per-connection. See docs/TRANSPORTS.md.",
        socket.display(),
        std::process::id()
    );

    loop {
        tokio::select! {
            biased;
            _ = signal::ctrl_c() => {
                eprintln!("nimvault-mcp: shutting down");
                let _ = std::fs::remove_file(&socket);
                remove_pidfile(&socket);
                break;
            }
            acc = listener.accept() => {
                let (stream, _) = acc?;
                let server = Server::new();
                tokio::spawn(async move {
                    match server.serve(stream).await {
                        Ok(service) => {
                            if let Err(e) = service.waiting().await {
                                eprintln!("nimvault-mcp: session ended: {e}");
                            }
                        }
                        Err(e) => eprintln!("nimvault-mcp: serve connection failed: {e}"),
                    }
                });
            }
        }
    }
    Ok(())
}

fn pidfile_path(socket: &Path) -> PathBuf {
    socket.with_extension("pid")
}

fn write_pidfile(socket: &Path) {
    let _ = std::fs::write(pidfile_path(socket), format!("{}\n", std::process::id()));
}

fn remove_pidfile(socket: &Path) {
    let _ = std::fs::remove_file(pidfile_path(socket));
}

pub fn default_socket_path() -> PathBuf {
    if let Ok(r) = std::env::var("XDG_RUNTIME_DIR") {
        return PathBuf::from(r).join("nimvault-mcp.sock");
    }
    dirs_fallback_runtime().join("nimvault-mcp.sock")
}

fn dirs_fallback_runtime() -> PathBuf {
    if let Ok(h) = std::env::var("HOME") {
        let p = PathBuf::from(h).join(".cache/nimvault-mcp");
        let _ = std::fs::create_dir_all(&p);
        return p;
    }
    std::env::temp_dir()
}

/// systemd user unit example (print only).
pub fn systemd_unit_example(socket: &Path) -> String {
    format!(
        "[Unit]\nDescription=nimvault-mcp Unix socket MCP\n\n[Service]\n\
ExecStart={bin} serve --socket {sock}\nRestart=on-failure\n\n[Install]\nWantedBy=default.target\n",
        bin = std::env::current_exe()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "nimvault-mcp".into()),
        sock = socket.display()
    )
}
