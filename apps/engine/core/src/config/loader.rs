//! Loading: defaults, then the file, then environment overrides, then secrets from the
//! environment.

use std::env;
use std::fs;
use std::path::PathBuf;

use serde_yaml::{Mapping, Value};

use crate::config::{ApiKey, Config, API_KEY_ENV};
use crate::error::ConfigError;

/// Prefix and separator for overrides. PIRAMID__RUNTIME__CACHE__MAX_BYTES=1024 sets
/// runtime.cache.max_bytes.
const ENV_PREFIX: &str = "PIRAMID__";
const ENV_SEPARATOR: &str = "__";

/// Read CONFIG_FILE, apply PIRAMID__ overrides, read PIRAMID_API_KEY and OPENAI_API_KEY, then
/// validate.
pub fn load() -> Result<Config, ConfigError> {
    let mut document = load_file()?;
    apply_env_overrides(&mut document)?;
    apply_secret_env(&mut document);

    let mut config: Config = serde_yaml::from_value(document).map_err(|e| {
        ConfigError::Invalid(format!(
            "{e}. Run `piramid show config` for the full set of keys"
        ))
    })?;
    config.startup.http.auth.api_key = server_api_key()?;

    config.validate().map_err(ConfigError::Invalid)?;
    Ok(config)
}

/// Parse CONFIG_FILE into an untyped document, or an empty one when it is unset.
fn load_file() -> Result<Value, ConfigError> {
    let Ok(path) = env::var("CONFIG_FILE") else {
        return Ok(Value::Mapping(Mapping::new()));
    };
    let data = fs::read_to_string(&path)
        .map_err(|e| ConfigError::File(format!("failed to read CONFIG_FILE '{path}': {e}")))?;

    let parsed = if path.ends_with(".yaml") || path.ends_with(".yml") {
        serde_yaml::from_str::<Value>(&data)
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
        let value = serde_yaml::from_str::<Value>(&raw).unwrap_or_else(|_| Value::String(raw));
        insert_at(document, &path, value).map_err(|reason| ConfigError::Env { name, reason })?;
    }
    Ok(())
}

/// Read the API key from the environment. It has no place in the configuration file.
fn apply_secret_env(document: &mut Value) {
    if let Ok(key) = env::var("OPENAI_API_KEY") {
        let path = ["startup", "embedding", "api_key"].map(str::to_string);
        let _ = insert_at(document, &path, Value::String(key));
    }
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

/// Default data directory: ~/.piramid. An unset HOME is an error.
pub fn default_data_dir() -> Result<String, ConfigError> {
    let home = env::var("HOME").map_err(|_| ConfigError::Env {
        name: "HOME".to_string(),
        reason: "unset, so the default data directory ~/.piramid cannot be resolved".to_string(),
    })?;
    let mut path = PathBuf::from(home);
    path.push(".piramid");
    Ok(path.to_string_lossy().to_string())
}
