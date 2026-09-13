//! The developer console: run every unit in the repo, stream its output, and watch the server,
//! from one modal terminal UI.
//!
//! Inside a checkout it drives just recipes and docker compose. Outside one it offers only the
//! views that need a server.

pub mod app;
pub mod client;
pub mod collections;
pub mod device;
mod health;
pub mod logs;
mod run;
pub mod runner;
pub mod settings;
pub mod types;
pub mod ui;
pub mod units;

pub use run::run;
