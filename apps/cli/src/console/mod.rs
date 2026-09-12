//! The developer console: run every unit in the repo, stream its output, and watch the server,
//! from one modal terminal UI.
//!
//! It drives just recipes and docker compose, so it runs only inside a checkout. Outside one it
//! prints help instead. Each action shells out to the recipe rather than reimplementing it.

mod app;
mod client;
mod collections;
mod device;
mod health;
mod logs;
mod run;
mod runner;
mod settings;
mod types;
mod ui;
mod units;

pub use run::run;
pub use settings::repo_root;
pub use types::Profile;

#[cfg(test)]
mod tests;
