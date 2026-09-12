//! Loading: defaults, then the file, then environment overrides, then secrets from the
//! environment.

use std::env;
use std::fs;
use std::path::Path;

use yaml_serde::{Mapping, Value};

use crate::config::{ApiKey, Config, API_KEY_ENV};
use crate::error::ConfigError;

/// Prefix and separator for overrides. PIRAMID__RUNTIME__WAL__MAX_LOG_SIZE=1024 sets
/// runtime.wal.max_log_size.
const ENV_PREFIX: &str = "PIRAMID__";
const ENV_SEPARATOR: &str = "__";

/// Read CONFIG_FILE, apply PIRAMID__ overrides, read PIRAMID_API_KEY and OPENAI_API_KEY, then
/// validate.
pub fn load() -> Result<Config, ConfigError> {
    load_with(None, |_| {})
}

/// Read the given file, or CONFIG_FILE when none is given, apply PIRAMID__ overrides, then
/// adjust the typed configuration and validate the result.
pub fn load_with(
    file: Option<&Path>,
    adjust: impl FnOnce(&mut Config),
) -> Result<Config, ConfigError> {
    let path = match file {
        Some(path) => Some(path.to_string_lossy().into_owned()),
        None => env::var("CONFIG_FILE").ok(),
    };
    let mut document = load_file(path)?;
    apply_env_overrides(&mut document)?;
    apply_secret_env(&mut document)?;

    let mut config: Config =
        yaml_serde::from_value(document).map_err(|e| ConfigError::Invalid(e.to_string()))?;
    adjust(&mut config);
    config.startup.http.auth.api_key = server_api_key()?;

    config.validate().map_err(ConfigError::Invalid)?;
    Ok(config)
}

/// Parse the configuration file into an untyped document, or an empty one when there is none.
fn load_file(path: Option<String>) -> Result<Value, ConfigError> {
    let Some(path) = path else {
        return Ok(Value::Mapping(Mapping::new()));
    };
    let data = fs::read_to_string(&path)
        .map_err(|e| ConfigError::File(format!("failed to read CONFIG_FILE '{path}': {e}")))?;

    let parsed = if path.ends_with(".yaml") || path.ends_with(".yml") {
        yaml_serde::from_str::<Value>(&data)
            .map_err(|e| ConfigError::File(format!("failed to parse YAML '{path}': {e}")))?
    } else if path.ends_with(".json") {
        serde_json::from_str::<Value>(&data)
            .map_err(|e| ConfigError::File(format!("failed to parse JSON '{path}': {e}")))?
    } else {
        return Err(ConfigError::File(format!(
            "unsupported CONFIG_FILE extension for '{path}', expected .yaml, .yml, or .json"
        )));
    };

    // An empty file parses as null, which is a valid document taking every default.
    Ok(match parsed {
        Value::Null => Value::Mapping(Mapping::new()),
        other => other,
    })
}

/// Merge every PIRAMID__ variable into the document at the path its name spells out.
fn apply_env_overrides(document: &mut Value) -> Result<(), ConfigError> {
    let mut overrides: Vec<(String, String)> = env::vars()
        .filter(|(name, _)| name.starts_with(ENV_PREFIX))
        .collect();
    // Overrides are applied in sorted order.
    overrides.sort();

    for (name, raw) in overrides {
        let path: Vec<String> = name[ENV_PREFIX.len()..]
            .split(ENV_SEPARATOR)
            .map(str::to_lowercase)
            .collect();
        if path.iter().any(String::is_empty) {
            return Err(ConfigError::Env {
                name: name.clone(),
                reason: "empty path segment".to_string(),
            });
        }
        // Values are parsed as YAML scalars. Anything that does not parse stays a string.
        let value = yaml_serde::from_str::<Value>(&raw).unwrap_or_else(|_| Value::String(raw));
        insert_at(document, &path, value).map_err(|reason| ConfigError::Env { name, reason })?;
    }
    Ok(())
}

/// Read the API key from the environment. It has no place in the configuration file.
fn apply_secret_env(document: &mut Value) -> Result<(), ConfigError> {
    if let Ok(key) = env::var("OPENAI_API_KEY") {
        let path = ["startup", "embedding", "api_key"].map(str::to_string);
        insert_at(document, &path, Value::String(key)).map_err(|reason| ConfigError::Env {
            name: "OPENAI_API_KEY".to_string(),
            reason,
        })?;
    }
    Ok(())
}

/// Read the server API key from the environment. It has no place in the configuration file.
fn server_api_key() -> Result<Option<ApiKey>, ConfigError> {
    let Ok(key) = env::var(API_KEY_ENV) else {
        return Ok(None);
    };
    ApiKey::new(key)
        .map(Some)
        .map_err(|reason| ConfigError::Env {
            name: API_KEY_ENV.to_string(),
            reason: format!("{reason}; unset it, or set it to the key clients must send"),
        })
}

/// Write the value at the given path, creating intermediate mappings.
fn insert_at(document: &mut Value, path: &[String], value: Value) -> Result<(), String> {
    let Some((leaf, parents)) = path.split_last() else {
        return Err("no path".to_string());
    };
    let mut cursor = document;
    for segment in parents {
        let key = Value::String(segment.clone());
        let Value::Mapping(map) = cursor else {
            return Err(format!("'{segment}' is not a section"));
        };
        cursor = map
            .entry(key)
            .or_insert_with(|| Value::Mapping(Mapping::new()));
    }
    let Value::Mapping(map) = cursor else {
        return Err(format!("'{leaf}' is not a section"));
    };
    map.insert(Value::String(leaf.clone()), value);
    Ok(())
}
