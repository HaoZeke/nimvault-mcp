//! Library surface for nimvault-mcp.

pub mod cli;
pub mod constants;

pub use cli::run_nimvault;
pub use constants::{VERSION, version_output};
