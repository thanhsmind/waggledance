//! waggledance-core — domain, application and adapters for the waggledance supervisor.

pub mod agent_usage;
pub mod ansi;
pub mod bee;
pub mod bee_upgrade;
pub mod config;
pub mod domain;
pub mod engine;
pub mod error;
pub mod intake;
pub mod notify_store;
pub mod paths_boundary;
pub mod process;
pub mod project_settings;
pub mod repository;
pub mod transcript;
pub mod usage;
pub mod usage_rows;
pub mod worktree_sweep;

pub use config::Config;
pub use engine::Engine;
pub use error::{Error, Result};
pub use repository::SqliteStore;
