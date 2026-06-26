//! nimvault-mcp library surface (testable edges).
//!
//! Binary modules (`main`, `server`, `setup`, …) live beside this crate root in
//! `src/` and are wired from `main.rs`. See `docs/ARCHITECTURE.md`.

pub mod constants;
pub mod doctor;
pub mod policy;

pub use constants::{VERSION, version_output};
pub use doctor::{install_help_block, server_instructions};
pub use policy::{blocked_mutate_message, enrich_error, ensure_mutate, push_recipient, trunc};
