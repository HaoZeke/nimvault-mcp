//! # nimvault-mcp
//!
//! Default: MCP over **stdio** (agent hosts). Optional: **`serve --socket`** for a
//! long-lived Unix-domain multi-client server — see `docs/TRANSPORTS.md`.

mod cli;
mod constants;
mod doctor;
mod policy;
#[cfg(unix)]
mod serve;
mod server;
mod session;
mod inproc;
mod setup;
mod tool_args;

use std::path::PathBuf;

use rmcp::ServiceExt;

use crate::server::Server;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("{}", constants::version_output());
        return Ok(());
    }
    if args.iter().any(|a| a == "setup") {
        setup::run(&args).await;
        return Ok(());
    }
    if args.iter().any(|a| a == "doctor") {
        let (cli_ok, detail) = cli::cli_identity().await;
        let gpg = if cli_ok {
            "Use MCP tool nimvault_doctor when connected."
        } else {
            ""
        };
        println!("{}", doctor::format_doctor_report(cli_ok, &detail, gpg));
        return Ok(());
    }
    if args.iter().any(|a| a == "serve") {
        #[cfg(unix)]
        {
            let socket = args
                .windows(2)
                .find(|w| w[0] == "--socket" || w[0] == "-s")
                .map(|w| PathBuf::from(&w[1]))
                .unwrap_or_else(serve::default_socket_path);
            if args.iter().any(|a| a == "--print-unit") {
                print!("{}", serve::systemd_unit_example(&socket));
                return Ok(());
            }
            doctor::emit_startup_stderr();
            return serve::run_unix_socket(socket).await;
        }
        #[cfg(not(unix))]
        {
            let _ = PathBuf::new();
            anyhow::bail!(
                "serve --socket (Unix domain MCP) is not supported on this OS; use stdio MCP"
            );
        }
    }

    doctor::emit_startup_stderr();
    let server = Server::new();
    let service = server.serve(rmcp::transport::io::stdio()).await?;
    service.waiting().await?;
    Ok(())
}
