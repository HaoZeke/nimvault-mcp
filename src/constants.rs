//! Shared constants.

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Optional explicit path to the `nimvault` binary.
pub fn nimvault_bin_env() -> Option<String> {
    std::env::var("NIMVAULT_BIN")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Default GPG recipient forwarded as `--recipient` when set (else CLI uses `.vault/config`).
pub fn default_recipient_env() -> Option<String> {
    std::env::var("NIMVAULT_GPG_RECIPIENT")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Default `repo_path` when tool args omit it (Grok/Claude/Codex can set per-server env).
pub fn default_repo_env() -> Option<String> {
    std::env::var("NIMVAULT_DEFAULT_REPO")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// When true, mutating tools (add/remove/seal/unseal) are allowed. Default off for safety.
pub fn mutate_enabled() -> bool {
    match std::env::var("NIMVAULT_MCP_ALLOW_MUTATE") {
        Ok(v) => {
            let v = v.trim().to_ascii_lowercase();
            v == "1" || v == "true" || v == "yes"
        }
        Err(_) => false,
    }
}

pub fn version_output() -> String {
    format!("nimvault-mcp {VERSION}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mutate_default_off() {
        if std::env::var("NIMVAULT_MCP_ALLOW_MUTATE").is_err() {
            assert!(!mutate_enabled());
        }
    }
}
