//! `nimvault-mcp setup` — configure MCP clients (Grok, Claude, Codex, add-mcp).

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

fn print_grok() {
    println!("\n=== Grok Build ===");
    println!("Marketplace (after PR lands): /marketplace → search nimvault → install");
    println!("Direct plugin (works today):");
    println!("  grok plugin install https://github.com/HaoZeke/nimvault-mcp.git --trust");
    println!("  # local checkout:");
    println!("  grok plugin install /path/to/nimvault-mcp --trust");
    println!("  # then Ctrl+L → Plugins → trust MCP; set env if needed in ~/.grok/config.toml:");
    println!("  # [mcp_servers.nimvault.env]");
    println!("  # NIMVAULT_DEFAULT_REPO = \"/home/you/.local/share/chezmoi\"");
    println!("  # NIMVAULT_GPG_RECIPIENT = \"YOUR_KEY_ID\"");
    println!("  # NIMVAULT_MCP_ALLOW_MUTATE = \"0\"");
    println!("Repo ships plugin.json, .mcp.json, skills/nimvault, scripts/run-mcp.sh");
    println!("Submission notes: MARKETPLACE.md");
}

fn print_claude() {
    println!("\n=== Claude Code / Desktop ===");
    println!("Project `.mcp.json` or user `~/.claude.json` → mcpServers:");
    println!(
        r#"  "nimvault": {{
    "type": "stdio",
    "command": "nimvault-mcp",
    "args": [],
    "env": {{
      "NIMVAULT_MCP_ALLOW_MUTATE": "0",
      "NIMVAULT_DEFAULT_REPO": "/path/to/repo/with/.vault",
      "NIMVAULT_GPG_RECIPIENT": "YOUR_KEY_ID"
    }}
  }}"#
    );
    println!("Claude Code CLI:");
    println!("  claude mcp add nimvault --env NIMVAULT_MCP_ALLOW_MUTATE=0 -- nimvault-mcp");
    println!("Allow tools in settings if using permission lists: mcp__nimvault__*");
}

fn print_codex() {
    println!("\n=== Codex CLI ===");
    println!("  codex mcp add nimvault \\");
    println!("    --env NIMVAULT_MCP_ALLOW_MUTATE=0 \\");
    println!("    --env NIMVAULT_DEFAULT_REPO=$HOME/.local/share/chezmoi \\");
    println!("    -- nimvault-mcp");
    println!("Or ~/.codex/config.toml:");
    println!(
        r#"  [mcp_servers.nimvault]
  command = "nimvault-mcp"
  startup_timeout_sec = 30
  [mcp_servers.nimvault.env]
  NIMVAULT_MCP_ALLOW_MUTATE = "0""#
    );
}

fn print_env() {
    println!("\n=== Environment ===");
    println!("  NIMVAULT_BIN                 path to nimvault CLI");
    println!("  NIMVAULT_GPG_RECIPIENT       default --recipient for mutate tools");
    println!("  NIMVAULT_DEFAULT_REPO        default repo_path when tools omit it");
    println!("  NIMVAULT_MCP_ALLOW_MUTATE    1/true enables add/remove/seal/unseal (default off)");
    println!("  NIMVAULT_GPG_PARALLEL        CLI parallelism (default 8) for seal/unseal");
    println!("  NIMVAULT_MCP_BIN             override MCP binary for scripts/run-mcp.sh");
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

fn wants(args: &[String], name: &str) -> bool {
    args.iter().any(|a| a == name || a == &format!("--{name}") || a == &format!("--client={name}"))
}

pub async fn run(args: &[String]) {
    println!("nimvault-mcp v{VERSION} — Setup\n");
    if which::which("nimvault").is_err() {
        eprintln!("warning: `nimvault` CLI not on PATH. Install: nimble install nimvault");
        eprintln!("         https://nimvault.rgoswami.me\n");
    }
    if find_binary().is_none() {
        eprintln!("warning: `nimvault-mcp` not on PATH. Install:");
        eprintln!("  cargo install --git https://github.com/HaoZeke/nimvault-mcp\n");
    }

    let all = args.len() <= 2
        || wants(args, "all")
        || (!wants(args, "grok") && !wants(args, "claude") && !wants(args, "codex") && !wants(args, "add-mcp"));

    let do_add = all || wants(args, "add-mcp");
    let mut ok = true;
    if do_add {
        ok = run_add_mcp();
    }

    if all || wants(args, "grok") {
        print_grok();
    }
    if all || wants(args, "claude") {
        print_claude();
    }
    if all || wants(args, "codex") {
        print_codex();
    }
    print_env();

    if do_add && !ok {
        println!("\nadd-mcp did not complete; use the client snippets above.");
        std::process::exit(1);
    }
    println!("\nRestart MCP clients or reload servers after changing config.");
}
