#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "assertions in tests"
)]
//! Configuration loading tests. Loading reads std::env, so these run under one lock and restore
//! the environment afterwards.

use std::sync::Mutex;

use piramid_core::config::loader;

static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Run body with vars set and CONFIG_FILE pointing at file, restoring the environment.
fn with_env<T>(file: Option<&str>, vars: &[(&str, &str)], body: impl FnOnce() -> T) -> T {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let path = file.map(|contents| {
        let path = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("loader_test.yaml");
        std::fs::write(&path, contents).unwrap();
        std::env::set_var("CONFIG_FILE", &path);
        path
    });
    for (name, value) in vars {
        std::env::set_var(name, value);
    }

    let result = body();

    for (name, _) in vars {
        std::env::remove_var(name);
    }
    std::env::remove_var("CONFIG_FILE");
    if let Some(path) = path {
        let _ = std::fs::remove_file(path);
    }
    result
}

#[test]
fn no_file_and_no_overrides_is_the_defaults() {
    let cfg = with_env(None, &[], || loader::load().unwrap());
    assert_eq!(cfg, piramid_core::config::Config::default());
}

#[test]
fn an_env_override_reaches_a_nested_key() {
    let cfg = with_env(
        None,
        &[("PIRAMID__RUNTIME__CACHE__METADATA__MAX_BYTES", "4096")],
        || loader::load().unwrap(),
    );
    assert_eq!(cfg.runtime.cache.metadata.max_bytes, Some(4096));
}

#[test]
fn env_values_parse_as_yaml_not_as_strings() {
    let cfg = with_env(
        None,
        &[
            ("PIRAMID__RUNTIME__WAL__ENABLED", "false"),
            ("PIRAMID__STARTUP__THREADS", "null"),
            ("PIRAMID__RUNTIME__SEARCH__FILTER_OVERFETCH", "7"),
            ("PIRAMID__STARTUP__LOGGING__LEVEL", "debug"),
        ],
        || loader::load().unwrap(),
    );
    assert!(!cfg.runtime.wal.enabled);
    assert_eq!(cfg.startup.threads, None);
    assert_eq!(cfg.runtime.search.filter_overfetch, 7);
    assert_eq!(
        cfg.startup.logging.level,
        piramid_core::config::LogLevel::Debug
    );
}

#[test]
fn an_override_wins_over_the_file() {
    let file = "startup:\n  bind: 127.0.0.1:1234\n";
    let cfg = with_env(
        Some(file),
        &[("PIRAMID__STARTUP__BIND", "127.0.0.1:9999")],
        || loader::load().unwrap(),
    );
    assert_eq!(cfg.startup.bind, "127.0.0.1:9999");
}

#[test]
fn an_unknown_override_is_an_error_naming_the_key() {
    let error = with_env(None, &[("PIRAMID__RUNTIME__NOT_A_KEY", "1")], || {
        loader::load().unwrap_err()
    });
    assert!(error.to_string().contains("not_a_key"), "{error}");
}

#[test]
fn an_invalid_value_fails_to_load() {
    let error = with_env(
        None,
        &[("PIRAMID__RUNTIME__SEARCH__FILTER_OVERFETCH", "0")],
        || loader::load().unwrap_err(),
    );
    assert!(error.to_string().contains("filter_overfetch"), "{error}");
}

#[test]
fn the_api_key_comes_from_the_environment_only() {
    let file = "startup:\n  embedding:\n    provider: openai\n    model: text-embedding-3-small\n";
    let cfg = with_env(Some(file), &[("OPENAI_API_KEY", "sk-test")], || {
        loader::load().unwrap()
    });
    assert_eq!(
        cfg.startup.embedding.unwrap().api_key.as_deref(),
        Some("sk-test")
    );
}
