#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "assertions in tests"
)]
//! Configuration loading tests. They run under one lock and restore the environment afterwards.

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

#[test]
fn the_server_api_key_comes_from_the_environment() {
    let cfg = with_env(None, &[("PIRAMID_API_KEY", "a-long-server-key")], || {
        loader::load().unwrap()
    });
    let key = cfg.startup.http.auth.api_key.unwrap();
    assert_eq!(key.expose(), "a-long-server-key");
    assert!(!format!("{key:?}").contains("a-long-server-key"));
}

#[test]
fn the_server_api_key_cannot_be_written_in_the_file() {
    let file = "startup:\n  http:\n    auth:\n      api_key: in-a-file\n";
    let error = with_env(Some(file), &[], || loader::load().unwrap_err());
    assert!(error.to_string().contains("api_key"), "{error}");
}

#[test]
fn an_empty_server_api_key_is_an_error() {
    let error = with_env(None, &[("PIRAMID_API_KEY", "")], || {
        loader::load().unwrap_err()
    });
    assert!(error.to_string().contains("PIRAMID_API_KEY"), "{error}");
}

#[test]
fn opting_out_of_authentication_while_setting_a_key_is_an_error() {
    let file = "startup:\n  http:\n    auth:\n      allow_unauthenticated: true\n";
    let error = with_env(
        Some(file),
        &[("PIRAMID_API_KEY", "a-long-server-key")],
        || loader::load().unwrap_err(),
    );
    assert!(
        error.to_string().contains("allow_unauthenticated"),
        "{error}"
    );
}

#[test]
fn a_zero_rate_limit_is_an_error() {
    let error = with_env(
        None,
        &[("PIRAMID__STARTUP__HTTP__RATE_LIMIT__BURST", "0")],
        || loader::load().unwrap_err(),
    );
    assert!(error.to_string().contains("burst"), "{error}");
}

// A source names its file directly and applies its port and data directory over the file.
#[test]
fn a_source_applies_its_values_over_its_file() {
    let path = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("loader_source.yaml");
    std::fs::write(
        &path,
        "startup:\n  bind: 127.0.0.1:6333\n  data_dir: ./from-file\n",
    )
    .unwrap();
    let cfg = with_env(None, &[], || {
        loader::load_from(&loader::ConfigSource {
            file: Some(path.clone()),
            port: Some(7000),
            data_dir: Some("/tmp/elsewhere".to_string()),
        })
        .unwrap()
    });
    assert_eq!(cfg.startup.bind, "127.0.0.1:7000");
    assert_eq!(cfg.startup.data_dir, "/tmp/elsewhere");

    let untouched = with_env(None, &[], || {
        loader::load_from(&loader::ConfigSource {
            file: Some(path.clone()),
            ..loader::ConfigSource::default()
        })
        .unwrap()
    });
    assert_eq!(untouched.startup.bind, "127.0.0.1:6333");
    assert_eq!(untouched.startup.data_dir, "./from-file");
}
