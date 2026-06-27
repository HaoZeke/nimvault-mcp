//! Long-lived local MCP server on a Unix domain socket (better than stdio-only).
//!
//! Each accepted connection runs a full MCP session over the stream (rmcp async-rw).
//! Socket mode 0o600. No network bind.

use std::path::PathBuf;

use anyhow::{Context, Result};
use rmcp::ServiceExt;
use tokio::net::UnixListener;

use crate::server::Server;

pub async fn run_unix_socket(socket: PathBuf) -> Result<()> {
    if socket.exists() {
        // Stale socket from crashed daemon
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

    eprintln!(
        "nimvault-mcp: listening on unix:{} (mode 0600). Ctrl-C to stop.\n\
         Hosts that only support stdio should keep spawning `nimvault-mcp` without `serve`.\n\
         See docs/TRANSPORTS.md.",
        socket.display()
    );

    loop {
        let (stream, _addr) = listener.accept().await?;
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

pub fn default_socket_path() -> PathBuf {
    if let Ok(r) = std::env::var("XDG_RUNTIME_DIR") {
        return PathBuf::from(r).join("nimvault-mcp.sock");
    }
    std::env::temp_dir().join(format!("nimvault-mcp-{}.sock", std::process::id()))
}
