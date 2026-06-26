//! Library surface for nimvault-mcp.

pub mod cli;
pub mod constants;
pub mod doctor;

pub use cli::run_nimvault;
pub use constants::{VERSION, version_output};
pub use doctor::{install_help_block, server_instructions};
