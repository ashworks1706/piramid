//! The developer console: runs every unit in the repo, streams its output, watches the server.

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
