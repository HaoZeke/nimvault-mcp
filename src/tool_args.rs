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
