//! Core (non-GUI) functionality for MarkLite.
//!
//! Keeping parsing and export logic in a library makes it reusable by the
//! desktop UI, a future CLI, and tests that do not need a graphical session.

pub mod export;
pub mod markdown;

pub const APP_NAME: &str = "MarkLite";
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
