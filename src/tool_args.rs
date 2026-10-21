//! MCP tool argument types.

use serde::Deserialize;

fn default_repo() -> Option<String> {
    None
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct RepoArgs {
    /// Git repository root that contains `.vault/`. Optional if NIMVAULT_DEFAULT_REPO is set or CWD walks up to a `.vault`.
    #[serde(default = "default_repo")]
    pub repo_path: Option<String>,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct PathRepoArgs {
    /// File path (absolute or as accepted by nimvault CLI).
    pub path: String,
    #[serde(default = "default_repo")]
    pub repo_path: Option<String>,
    #[serde(default)]
    pub recipient: Option<String>,
    #[serde(default)]
    pub no_gitignore: Option<bool>,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct MoveArgs {
    /// Current target path of the vault entry.
    pub old_path: String,
    /// New target path.
    pub new_path: String,
    #[serde(default = "default_repo")]
    pub repo_path: Option<String>,
    #[serde(default)]
    pub recipient: Option<String>,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct SealArgs {
    #[serde(default = "default_repo")]
    pub repo_path: Option<String>,
    #[serde(default)]
    pub recipient: Option<String>,
    /// Re-encrypt every entry. Pass false to seal only plaintext that changed.
    pub force: bool,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct RotateArgs {
    #[serde(default = "default_repo")]
    pub repo_path: Option<String>,
    #[serde(default)]
    pub recipient: Option<String>,
    /// Re-encrypt every payload under a fresh data key.
    /// Pass false to rewrap data keys only.
    pub rekey: bool,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct UnsealArgs {
    #[serde(default = "default_repo")]
    pub repo_path: Option<String>,
    #[serde(default)]
    pub allow_unsigned: Option<bool>,
    #[serde(default)]
    pub recipient: Option<String>,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct ScanArgs {
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default = "default_repo")]
    pub repo_path: Option<String>,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct EmptyArgs {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seal_force_is_required() {
        assert!(serde_json::from_str::<SealArgs>(r#"{"repo_path":"/tmp/v"}"#).is_err());
        let ok = serde_json::from_str::<SealArgs>(r#"{"force":false}"#).unwrap();
        assert!(!ok.force);
        let forced = serde_json::from_str::<SealArgs>(r#"{"force":true}"#).unwrap();
        assert!(forced.force);
    }

    #[test]
    fn rotate_rekey_is_required() {
        assert!(serde_json::from_str::<RotateArgs>(r#"{}"#).is_err());
        let wrap = serde_json::from_str::<RotateArgs>(r#"{"rekey":false}"#).unwrap();
        assert!(!wrap.rekey);
        let rekey = serde_json::from_str::<RotateArgs>(r#"{"rekey":true}"#).unwrap();
        assert!(rekey.rekey);
    }
}
