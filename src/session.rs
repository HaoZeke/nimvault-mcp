//! Per-MCP-connection / per-process session state (sticky vault root).

use std::path::PathBuf;
use std::sync::{Arc, RwLock};

/// Shared across tool calls in one Server instance (one stdio session or one UDS connection).
#[derive(Clone, Default)]
pub struct Session {
    inner: Arc<RwLock<SessionInner>>,
}

#[derive(Default)]
struct SessionInner {
    /// Sticky repo root once resolved (explicit, env, or walk-up).
    vault_root: Option<PathBuf>,
}

impl Session {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(SessionInner::default())),
        }
    }

    pub fn sticky_root(&self) -> Option<PathBuf> {
        self.inner.read().ok().and_then(|g| g.vault_root.clone())
    }

    pub fn remember_root(&self, path: PathBuf) {
        if let Ok(mut g) = self.inner.write() {
            g.vault_root = Some(path);
        }
    }

    /// Prefer explicit arg, then sticky session, then env/walk-up via caller.
    pub fn prefer_repo_arg(&self, repo_path: &Option<String>) -> Option<String> {
        if let Some(p) = repo_path
            .as_ref()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
        {
            return Some(p);
        }
        self.sticky_root()
            .map(|p| p.to_string_lossy().into_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sticky_remembers() {
        let s = Session::new();
        assert!(s.sticky_root().is_none());
        s.remember_root(PathBuf::from("/tmp/vaultrepo"));
        assert_eq!(
            s.sticky_root().unwrap(),
            PathBuf::from("/tmp/vaultrepo")
        );
        let pref = s.prefer_repo_arg(&None);
        assert_eq!(pref.unwrap(), "/tmp/vaultrepo");
        let explicit = s.prefer_repo_arg(&Some("/other".into()));
        assert_eq!(explicit.unwrap(), "/other");
    }
}
