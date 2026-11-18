//! Persistenza locale: `config.toml` (§15) e database SQLite (§5).
#![cfg_attr(test, allow(clippy::unwrap_used))]

pub mod config;
pub mod db;
pub mod schema;

pub use config::{Config, ConfigError, ConfigFile};
pub use db::Db;
