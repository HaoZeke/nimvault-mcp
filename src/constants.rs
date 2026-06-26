//! Shared constants and policy flags.

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn nimvault_bin_env() -> Option<String> {
    std::env::var("NIMVAULT_BIN")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub fn default_recipient_env() -> Option<String> {
    std::env::var("NIMVAULT_GPG_RECIPIENT")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub fn default_repo_env() -> Option<String> {
    std::env::var("NIMVAULT_DEFAULT_REPO")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Hard read-only: even NIMVAULT_MCP_ALLOW_MUTATE cannot enable writes.
pub fn read_only_locked() -> bool {
    match std::env::var("NIMVAULT_MCP_READ_ONLY") {
        Ok(v) => {
            let v = v.trim().to_ascii_lowercase();
            v == "1" || v == "true" || v == "yes"
        }
        Err(_) => false,
    }
}

/// Mutating tools allowed only if not read-only locked and ALLOW_MUTATE set.
pub fn mutate_enabled() -> bool {
    if read_only_locked() {
        return false;
    }
    match std::env::var("NIMVAULT_MCP_ALLOW_MUTATE") {
        Ok(v) => {
            let v = v.trim().to_ascii_lowercase();
            v == "1" || v == "true" || v == "yes"
        }
        Err(_) => false,
    }
}

/// Append-only audit file path (tool name + repo + paths; never secret bodies).
pub fn audit_log_path() -> Option<String> {
    std::env::var("NIMVAULT_MCP_AUDIT_LOG")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub fn version_output() -> String {
    format!("nimvault-mcp {VERSION}")
}
