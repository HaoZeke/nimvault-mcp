//! nimvault-mcp library surface (testable edges).
//! See `docs/ARCHITECTURE.md` and `docs/TRANSPORTS.md`.

pub mod constants;
pub mod doctor;
pub mod inproc;
pub mod policy;

pub use constants::{VERSION, version_output};
pub use doctor::{install_help_block, server_instructions};
pub use policy::{blocked_mutate_message, enrich_error, ensure_mutate, push_recipient, trunc};
