//! # nimvault-mcp
//!
//! MCP server for [nimvault](https://github.com/HaoZeke/nimvault) — GPG-encrypted
//! opaque-blob vault operations for agents (list, status, scan, gated mutate).

mod cli;
mod constants;
mod server;
mod setup;
mod tool_args;

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

    if which::which("nimvault").is_err() && constants::nimvault_bin_env().is_none() {
        eprintln!(
            "nimvault-mcp: `nimvault` not on PATH (and NIMVAULT_BIN unset).              Install with `nimble install nimvault`."
        );
    }

    let server = Server::new();
    let service = server.serve(rmcp::transport::io::stdio()).await?;
    service.waiting().await?;
    Ok(())
}
