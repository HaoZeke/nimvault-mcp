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
    msg
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
}
