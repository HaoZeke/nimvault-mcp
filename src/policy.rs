//! Mutate / read-only gates and CLI argv helpers (pure policy, no process I/O).

use crate::constants::{default_recipient_env, mutate_enabled, read_only_locked};
use crate::doctor::install_help_block;

pub fn blocked_mutate_message() -> String {
    if read_only_locked() {
        return "BLOCKED: NIMVAULT_MCP_READ_ONLY=1 — all mutating tools disabled (hard lock). \
list/status/scan/doctor/resolve_repo only. Call nimvault_doctor."
            .into();
    }
    "BLOCKED: set NIMVAULT_MCP_ALLOW_MUTATE=1 in MCP env to enable add/remove/mv/seal/unseal \
(writes `.vault/`; never returns secret file bodies). list/status/scan remain available. \
Call nimvault_doctor. See docs/SURVEY.md and docs/ARCHITECTURE.md."
        .into()
}

pub fn ensure_mutate() -> Result<(), String> {
    if mutate_enabled() {
        Ok(())
    } else {
        Err(blocked_mutate_message())
    }
}

pub fn enrich_error(e: impl AsRef<str>) -> String {
    let mut msg = format!("ERROR: {}", e.as_ref());
    let s = e.as_ref();
    if s.contains("not found") || s.contains("NIMVAULT_BIN") || s.contains("No repo_path") {
        msg.push_str(&install_help_block());
    }
    agent_view(&msg)
}

pub fn push_recipient(args: &mut Vec<String>, recipient: &Option<String>) {
    let r = recipient
        .as_ref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(default_recipient_env);
    if let Some(r) = r {
        args.push("--recipient".into());
        args.push(r);
    }
}

pub fn trunc(s: &str, max: usize) -> String {
    let t = s.trim();
    if t.len() <= max {
        t.to_string()
    } else {
        format!("{}\n… truncated ({} bytes total)", &t[..max], t.len())
    }
}

/// What the agent, the model, and the web may see.
///
/// Allowed: paths, opaque ids, rule names, line numbers, byte sizes,
/// sync state, recipient ids. Forbidden: file bodies, PEM, token
/// literals, DEKs. Every MCP tool result goes through this, including
/// FAILED paths.
pub fn agent_view(s: &str) -> String {
    let mut out = String::new();
    let mut in_pem = false;
    let mut pem_bytes = 0usize;
    for line in s.lines() {
        let t = line.trim();
        if t.starts_with("-----BEGIN ") && t.ends_with("-----") {
            in_pem = true;
            pem_bytes = 0;
            continue;
        }
        if in_pem {
            if t.starts_with("-----END ") {
                in_pem = false;
                out.push_str(&format!("[redacted pem {pem_bytes} bytes]\n"));
            } else {
                pem_bytes += t.len();
            }
            continue;
        }
        if looks_like_secret(t) {
            out.push_str(&format!("[redacted {} bytes]\n", t.len()));
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    if in_pem {
        out.push_str(&format!("[redacted pem {pem_bytes} bytes]\n"));
    }
    out
}

fn looks_like_secret(t: &str) -> bool {
    if t.is_empty() {
        return false;
    }
    // Integrity hashes and 16-char vault ids are shape, not contents.
    if is_hex(t, 16) || is_hex(t, 40) || is_hex(t, 64) {
        return false;
    }
    const PREFIXES: &[&str] = &[
        "sk-",
        "sk_ant",
        "ghp_",
        "gho_",
        "github_pat_",
        "glpat-",
        "xoxb-",
        "xoxp-",
        "AKIA",
        "ASIA",
        "cio_",
        "npm_",
    ];
    if PREFIXES.iter().any(|p| t.contains(p)) {
        return true;
    }
    if let Some((_, v)) = t.split_once('=') {
        let v = v.trim().trim_matches(|c| c == '"' || c == '\'');
        if v.len() >= 20 && looks_token(v) {
            return true;
        }
    }
    looks_token(t)
}

fn is_hex(t: &str, n: usize) -> bool {
    t.len() == n && t.chars().all(|c| c.is_ascii_hexdigit())
}

fn looks_token(t: &str) -> bool {
    if t.len() < 32 {
        return false;
    }
    if is_hex(t, t.len()) && (t.len() == 16 || t.len() == 40 || t.len() == 64) {
        return false;
    }
    let keep = t
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '+' | '=' | '-' | '_'))
        .count();
    keep * 10 >= t.len() * 9
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trunc_short() {
        assert_eq!(trunc("hi", 10), "hi");
    }

    #[test]
    fn trunc_long() {
        let s = trunc(&"x".repeat(100), 10);
        assert!(s.contains("truncated"));
    }

    #[test]
    fn agent_view_redacts_pem() {
        let s = "keep\n-----BEGIN OPENSSH PRIVATE KEY-----\nabcDEF123==\n-----END OPENSSH PRIVATE KEY-----\npath=/tmp/id\n";
        let v = agent_view(s);
        assert!(v.contains("keep"));
        assert!(v.contains("[redacted pem"));
        assert!(!v.contains("abcDEF123"));
        assert!(v.contains("path=/tmp/id"));
    }

    #[test]
    fn agent_view_redacts_token_line() {
        let s = "export OPENAI_API_KEY=sk-abcdefghijklmnopqrstuvwxyz012345\n";
        let v = agent_view(s);
        assert!(v.contains("[redacted"));
        assert!(!v.contains("sk-abcdefghijklmnopqrstuvwxyz012345"));
    }

    #[test]
    fn agent_view_keeps_ids_and_sizes() {
        let s = "  deadbeefdeadbeef  secrets/a.txt\n  [in-sync] secrets/a.txt\n  [ssh-private-key] secrets/id:3 bytes=411\n";
        let v = agent_view(s);
        assert!(v.contains("deadbeefdeadbeef"));
        assert!(v.contains("bytes=411"));
        assert!(v.contains("[in-sync]"));
    }
}
