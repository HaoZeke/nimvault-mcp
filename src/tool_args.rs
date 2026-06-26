//! MCP tool argument types.

use serde::Deserialize;

fn default_repo() -> Option<String> {
    None
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct RepoArgs {
    /// Git repository root that contains `.vault/` (nimvault is CWD-sensitive). Defaults to server CWD.
    #[serde(default = "default_repo")]
    pub repo_path: Option<String>,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct PathRepoArgs {
    /// File path to add/remove (absolute or as accepted by nimvault CLI).
    pub path: String,
    #[serde(default = "default_repo")]
    pub repo_path: Option<String>,
    /// Optional GPG recipient (overrides `.vault/config` / NIMVAULT_GPG_RECIPIENT).
    #[serde(default)]
    pub recipient: Option<String>,
    /// If true, do not auto-append the path to `.gitignore` on add.
    #[serde(default)]
    pub no_gitignore: Option<bool>,
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
    /// Allow legacy unsigned vault blobs (fail-open migration).
    #[serde(default)]
    pub allow_unsigned: Option<bool>,
    #[serde(default)]
    pub recipient: Option<String>,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct ScanArgs {
    /// Directory to scan for unvaulted secrets (defaults to repo_path or CWD).
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default = "default_repo")]
    pub repo_path: Option<String>,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct EmptyArgs {}
