//! Core (non-GUI) functionality for MarkLite.
//!
//! Keeping parsing and export logic in a library makes it reusable by the
//! desktop UI, a future CLI, and tests that do not need a graphical session.

pub mod export;
pub mod markdown;

pub const APP_NAME: &str = "MarkLite";
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const AUTHOR_NAME: &str = "Carl Lau";
pub const AUTHOR_EMAIL: &str = "carllau80@gmail.com";
pub const GITHUB_URL: &str = "https://github.com/carllau80-ux/markdown-lite-show";
