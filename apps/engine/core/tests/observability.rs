#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "assertions in tests"
)]
//! Log filter construction from the logging configuration.

use piramid_core::config::{LogLevel, LoggingConfig};
use piramid_core::observability::{directives, filter_for, level_directive};

#[test]
fn defaults_produce_a_bare_level() {
    assert_eq!(directives("info", LoggingConfig::default()), "info");
}

#[test]
fn a_disabled_subsystem_becomes_an_off_directive() {
    let logging = LoggingConfig {
        search: false,
        http: false,
        ..LoggingConfig::default()
    };
    assert_eq!(
        directives("info", logging),
        "info,piramid::search=off,piramid::http=off"
    );
}

#[test]
fn the_filter_comes_from_the_configuration_only() {
    std::env::set_var("RUST_LOG", "trace");
    let logging = LoggingConfig {
        level: LogLevel::Warn,
        indexing: false,
        ..LoggingConfig::default()
    };
    let filter = filter_for(logging);
    std::env::remove_var("RUST_LOG");
    let filter = filter.unwrap().to_string();
    assert!(filter.contains("warn"), "{filter}");
    assert!(!filter.contains("trace"), "{filter}");
    assert!(filter.contains("piramid::indexing=off"), "{filter}");
}

#[test]
fn every_level_maps_to_a_tracing_name() {
    for (level, name) in [
        (LogLevel::Error, "error"),
        (LogLevel::Warn, "warn"),
        (LogLevel::Info, "info"),
        (LogLevel::Debug, "debug"),
        (LogLevel::Trace, "trace"),
    ] {
        assert_eq!(level_directive(level), name);
    }
}
