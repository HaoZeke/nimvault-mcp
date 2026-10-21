//! `nimvault-mcp setup` — configure MCP clients (Grok, Claude, Codex, add-mcp).
//!
//! A second run leaves an existing nimvault entry byte-for-byte unchanged.
//! add-mcp rewrites client files, so it runs only with `--add-mcp` and only
//! when the nimvault entry is not already there.

use std::path::{Path, PathBuf};

use crate::constants::VERSION;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Client {
    Claude,
    Codex,
}

fn config_home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
}

fn mcp_command() -> String {
    if let Ok(p) = which::which("nimvault-mcp") {
        return p.to_string_lossy().into_owned();
    }
    if let Ok(p) = std::env::current_exe() {
        return p.to_string_lossy().into_owned();
    }
    "nimvault-mcp".into()
}

fn client_path(home: &Path, client: Client) -> PathBuf {
    match client {
        Client::Claude => home.join(".claude.json"),
        Client::Codex => home.join(".codex").join("config.toml"),
    }
}

fn claude_has_server(path: &Path) -> bool {
    let Ok(text) = std::fs::read_to_string(path) else {
        return false;
    };
    if text.trim().is_empty() {
        return false;
    }
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else {
        return false;
    };
    v.pointer("/mcpServers/nimvault").is_some()
}

fn toml_header_is_nimvault(line: &str) -> bool {
    let t = line.trim();
    let t = t.split('#').next().unwrap_or("").trim();
    t == "[mcp_servers.nimvault]"
}

fn toml_has_server(path: &Path) -> bool {
    let Ok(text) = std::fs::read_to_string(path) else {
        return false;
    };
    text.lines().any(toml_header_is_nimvault)
}

fn client_configured(home: &Path, client: Client) -> bool {
    let path = client_path(home, client);
    match client {
        Client::Claude => claude_has_server(&path),
        Client::Codex => toml_has_server(&path),
    }
}

fn nimvault_claude_entry(command: &str) -> serde_json::Value {
    serde_json::json!({
        "type": "stdio",
        "command": command,
        "args": [],
        "env": {
            "NIMVAULT_MCP_ALLOW_MUTATE": "0"
        }
    })
}

fn apply_claude(path: &Path, command: &str) -> std::io::Result<bool> {
    if claude_has_server(path) {
        return Ok(false);
    }
    let mut root = if path.exists() {
        let text = std::fs::read_to_string(path)?;
        if text.trim().is_empty() {
            serde_json::json!({})
        } else {
            serde_json::from_str::<serde_json::Value>(&text)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?
        }
    } else {
        serde_json::json!({})
    };
    let obj = root.as_object_mut().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "claude config is not an object",
        )
    })?;
    let servers = obj
        .entry("mcpServers")
        .or_insert_with(|| serde_json::json!({}));
    let servers = servers.as_object_mut().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "mcpServers is not an object",
        )
    })?;
    servers.insert("nimvault".into(), nimvault_claude_entry(command));
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut body = serde_json::to_string_pretty(&root)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    body.push('\n');
    std::fs::write(path, body)?;
    Ok(true)
}

fn toml_block(command: &str) -> String {
    let cmd = command.replace('\\', "\\\\").replace('"', "\\\"");
    format!(
        "[mcp_servers.nimvault]\ncommand = \"{cmd}\"\nargs = []\nstartup_timeout_sec = 30\n\n[mcp_servers.nimvault.env]\nNIMVAULT_MCP_ALLOW_MUTATE = \"0\"\n"
    )
}

fn apply_toml(path: &Path, command: &str) -> std::io::Result<bool> {
    if toml_has_server(path) {
        return Ok(false);
    }
    let existing = if path.exists() {
        std::fs::read_to_string(path)?
    } else {
        String::new()
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let block = toml_block(command);
    let body = if existing.is_empty() {
        block
    } else {
        let mut out = existing;
        if !out.ends_with('\n') {
            out.push('\n');
        }
        out.push('\n');
        out.push_str(&block);
        out
    };
    std::fs::write(path, body)?;
    Ok(true)
}

fn apply_client(home: &Path, client: Client, command: &str) -> std::io::Result<bool> {
    let path = client_path(home, client);
    match client {
        Client::Claude => apply_claude(&path, command),
        Client::Codex => apply_toml(&path, command),
    }
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
    println!("setup writes ~/.claude.json when mcpServers.nimvault is absent.");
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
    println!("setup writes ~/.codex/config.toml when [mcp_servers.nimvault] is absent.");
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
    println!(
        "  NIMVAULT_MCP_ALLOW_MUTATE    1/true enables add/remove/seal/rotate/unseal (default off)"
    );
    println!("  NIMVAULT_GPG_PARALLEL        CLI parallelism (default 8) for seal/unseal");
    println!("  NIMVAULT_MCP_BIN             override MCP binary for scripts/run-mcp.sh");
}

fn wants(args: &[String], name: &str) -> bool {
    args.iter()
        .any(|a| a == name || a == &format!("--{name}") || a == &format!("--client={name}"))
}

fn explicit_client(args: &[String]) -> bool {
    wants(args, "grok") || wants(args, "claude") || wants(args, "codex")
}

fn clients_for(args: &[String]) -> Vec<Client> {
    let all = wants(args, "all") || !explicit_client(args);
    let mut v = Vec::new();
    if all || wants(args, "claude") {
        v.push(Client::Claude);
    }
    if all || wants(args, "codex") {
        v.push(Client::Codex);
    }
    v
}

struct SavedFile {
    path: PathBuf,
    bytes: Vec<u8>,
}

/// Bytes of client files that already name nimvault. add-mcp rewrites whole
/// files, so these are written back after it runs.
fn snapshot_configured(home: &Path, clients: &[Client]) -> Vec<SavedFile> {
    let mut saved = Vec::new();
    for client in clients {
        if !client_configured(home, *client) {
            continue;
        }
        let path = client_path(home, *client);
        if let Ok(bytes) = std::fs::read(&path) {
            saved.push(SavedFile { path, bytes });
        }
    }
    saved
}

fn restore_files(saved: &[SavedFile]) {
    for file in saved {
        if let Some(parent) = file.path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&file.path, &file.bytes);
    }
}

fn run_add_mcp(command: &str) -> bool {
    let mut cmd = std::process::Command::new("npx");
    cmd.args([
        "-y", "add-mcp", command, "--name", "nimvault", "-y", "--all",
    ]);
    println!("Running: npx -y add-mcp {command} --name nimvault -y --all");
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

fn client_label(client: Client) -> &'static str {
    match client {
        Client::Claude => "claude",
        Client::Codex => "codex",
    }
}

pub async fn run(args: &[String]) {
    println!("nimvault-mcp v{VERSION} — Setup\n");
    if which::which("nimvault").is_err() {
        eprintln!("warning: `nimvault` CLI not on PATH. Install: nimble install nimvault");
        eprintln!("         https://nimvault.rgoswami.me\n");
    }
    if which::which("nimvault-mcp").is_err() {
        eprintln!("warning: `nimvault-mcp` not on PATH. Install:");
        eprintln!("  cargo install --git https://github.com/HaoZeke/nimvault-mcp\n");
    }

    let Some(home) = config_home() else {
        eprintln!("setup: HOME is not set");
        std::process::exit(1);
    };
    let command = mcp_command();
    let clients = clients_for(args);
    let do_add = wants(args, "add-mcp");
    let missing_before = clients.iter().any(|c| !client_configured(&home, *c));
    if do_add && missing_before {
        let saved = snapshot_configured(&home, &clients);
        if !run_add_mcp(&command) {
            eprintln!("warning: add-mcp did not complete; writing client config directly");
        }
        restore_files(&saved);
    }

    let mut failed = false;
    for client in &clients {
        match apply_client(&home, *client, &command) {
            Ok(true) => println!(
                "{}: wrote {}",
                client_label(*client),
                client_path(&home, *client).display()
            ),
            Ok(false) => println!("{}: unchanged", client_label(*client)),
            Err(e) => {
                eprintln!("{}: {e}", client_label(*client));
                failed = true;
            }
        }
    }

    let all = wants(args, "all") || !explicit_client(args);
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
    println!("\nA second setup leaves an existing nimvault entry unchanged.");
    println!("Restart MCP clients or reload servers after changing config.");

    if failed {
        std::process::exit(1);
    }
    if do_add && clients.iter().any(|c| !client_configured(&home, *c)) {
        eprintln!("\nadd-mcp did not complete; use the client snippets above.");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_home(label: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "nv_mcp_setup_{label}_{}_{}",
            std::process::id(),
            label
        ));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn claude_and_codex_writes_are_idempotent() {
        let home = temp_home("idem");
        assert!(apply_client(&home, Client::Claude, "nimvault-mcp").unwrap());
        let claude = std::fs::read(home.join(".claude.json")).unwrap();
        assert!(!apply_client(&home, Client::Claude, "other-bin").unwrap());
        assert_eq!(claude, std::fs::read(home.join(".claude.json")).unwrap());
        let v: serde_json::Value = serde_json::from_slice(&claude).unwrap();
        assert_eq!(
            v.pointer("/mcpServers/nimvault/command")
                .and_then(|c| c.as_str()),
            Some("nimvault-mcp")
        );
        assert!(v.pointer("/mcpServers/nimvault2").is_none());

        let pre = "existing = true\n";
        std::fs::create_dir_all(home.join(".codex")).unwrap();
        std::fs::write(home.join(".codex/config.toml"), pre).unwrap();
        assert!(apply_client(&home, Client::Codex, "nimvault-mcp").unwrap());
        let codex = std::fs::read(home.join(".codex/config.toml")).unwrap();
        assert!(!apply_client(&home, Client::Codex, "other-bin").unwrap());
        assert_eq!(
            codex,
            std::fs::read(home.join(".codex/config.toml")).unwrap()
        );
        let text = String::from_utf8(codex).unwrap();
        assert!(text.starts_with(pre));
        assert_eq!(text.matches("[mcp_servers.nimvault]").count(), 1, "{text}");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn existing_command_is_left_alone() {
        let home = temp_home("keep");
        let path = home.join(".claude.json");
        std::fs::write(
            &path,
            "{\n  \"mcpServers\": {\n    \"nimvault\": {\n      \"command\": \"custom-bin\"\n    }\n  }\n}\n",
        )
        .unwrap();
        let before = std::fs::read(&path).unwrap();
        assert!(!apply_client(&home, Client::Claude, "nimvault-mcp").unwrap());
        assert_eq!(before, std::fs::read(&path).unwrap());
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn commented_codex_header_is_left_alone() {
        let home = temp_home("comment");
        std::fs::create_dir_all(home.join(".codex")).unwrap();
        let body = "[mcp_servers.nimvault] # custom\ncommand = \"custom-bin\"\n";
        let path = home.join(".codex/config.toml");
        std::fs::write(&path, body).unwrap();
        assert!(!apply_client(&home, Client::Codex, "nimvault-mcp").unwrap());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), body);
        let _ = std::fs::remove_dir_all(&home);
    }
}
