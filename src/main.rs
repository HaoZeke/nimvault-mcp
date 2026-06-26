//! # nimvault-mcp
//!
//! MCP server for [nimvault](https://github.com/HaoZeke/nimvault). Design: `docs/ARCHITECTURE.md`.

mod cli;
mod constants;
mod doctor;
mod policy;
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
    if args.iter().any(|a| a == "doctor") {
        let (cli_ok, detail) = match which::which("nimvault") {
            Ok(p) => (true, p.display().to_string()),
            Err(_) => (
                false,
                "not on PATH (set NIMVAULT_BIN or install nimvault)".into(),
            ),
        };
        println!(
            "{}",
            doctor::format_doctor_report(
                cli_ok,
                &detail,
                "Use MCP tool nimvault_doctor when connected."
            )
        );
        return Ok(());
    }

    doctor::emit_startup_stderr();

    let server = Server::new();
    let service = server.serve(rmcp::transport::io::stdio()).await?;
    service.waiting().await?;
    Ok(())
}
