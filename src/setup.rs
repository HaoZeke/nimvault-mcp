//! `nimvault-mcp setup` — configure MCP clients.

use crate::constants::VERSION;

fn find_binary() -> Option<String> {
    let output = std::process::Command::new("which")
        .arg("nimvault-mcp")
        .output()
        .ok()?;
    if output.status.success() {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() {
            return Some(path);
        }
    }
    None
}

fn print_guides() {
    println!("
--- Client-specific install ---");
    println!("
Grok Build (plugin):");
    println!("  grok plugin install https://github.com/HaoZeke/nimvault-mcp.git");
    println!("  # set NIMVAULT_MCP_ALLOW_MUTATE=1 only if agents may seal/add/unseal");
    println!("  # set NIMVAULT_GPG_RECIPIENT when `.vault/config` is missing");

    println!("
Claude / Cursor (manual JSON):");
    println!(
        r#"  {{
    "mcpServers": {{
      "nimvault": {{
        "command": "npx",
        "args": ["-y", "@haozeke/nimvault-mcp"],
        "env": {{
          "NIMVAULT_MCP_ALLOW_MUTATE": "0"
        }}
      }}
    }}
  }}"#
    );

    println!("
Codex CLI:");
    println!("  codex mcp add nimvault -- npx -y @haozeke/nimvault-mcp");

    println!("
Env knobs:");
    println!("  NIMVAULT_BIN                 path to nimvault CLI");
    println!("  NIMVAULT_GPG_RECIPIENT       default --recipient for mutate tools");
    println!("  NIMVAULT_MCP_ALLOW_MUTATE    1/true enables add/remove/seal/unseal (default off)");
}

fn run_add_mcp() -> bool {
    let target = if let Some(bin_path) = find_binary() {
        bin_path
    } else {
        "npx -y @haozeke/nimvault-mcp".to_string()
    };
    let mut cmd = std::process::Command::new("npx");
    cmd.args(["-y", "add-mcp", &target, "--name", "nimvault", "-y", "--all"]);
    println!("Running: npx -y add-mcp {target} --name nimvault -y --all");
    match cmd.status() {
        Ok(s) if s.success() => true,
        Ok(s) => {
            eprintln!("add-mcp exited with {s}");
            false
        }
        Err(e) => {
            eprintln!("add-mcp failed ({e}). Is Node.js/npx installed?");
            false
        }
    }
}

pub async fn run(_args: &[String]) {
    println!("nimvault-mcp v{VERSION} -- Setup
");
    if which::which("nimvault").is_err() {
        eprintln!("warning: `nimvault` not on PATH. Install with `nimble install nimvault`.");
    }
    let ok = run_add_mcp();
    print_guides();
    if !ok {
        std::process::exit(1);
    }
    println!("
Setup finished. Restart MCP clients or reload servers.");
}
