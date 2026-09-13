//! Loading: defaults, then the file, then environment overrides, then secrets from the
//! environment.

use std::env::{self, VarError};
use std::fs;
use std::path::PathBuf;

use yaml_serde::{Mapping, Value};

use crate::config::{ApiKey, Config, API_KEY_ENV};
use crate::error::ConfigError;

/// Name of the environment variable holding the key for the openai embedding provider.
const OPENAI_API_KEY_ENV: &str = "OPENAI_API_KEY";

/// Prefix and separator for overrides. PIRAMID__RUNTIME__WAL__MAX_LOG_SIZE=1024 sets
/// runtime.wal.max_log_size.
const ENV_PREFIX: &str = "PIRAMID__";
const ENV_SEPARATOR: &str = "__";

/// Where configuration comes from: a file, and command-line values applied over it.
///
/// A reload reads the same source the server booted from.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConfigSource {
    /// The file to read. None reads CONFIG_FILE, or no file when that is unset.
    pub file: Option<PathBuf>,
    /// Replaces the port of startup.bind and keeps its host.
    pub port: Option<u16>,
    /// Replaces startup.data_dir.
    pub data_dir: Option<String>,
}

/// Read CONFIG_FILE, apply PIRAMID__ overrides, read PIRAMID_API_KEY, and OPENAI_API_KEY for the
/// openai provider, then validate.
pub fn load() -> Result<Config, ConfigError> {
    load_from(&ConfigSource::default())
}

/// Read the file of a source, apply PIRAMID__ overrides and the values of the source, read the
/// secrets from the environment, then validate.
pub fn load_from(source: &ConfigSource) -> Result<Config, ConfigError> {
    let path = match &source.file {
        Some(path) => Some(path.clone()),
        None => env::var_os("CONFIG_FILE").map(PathBuf::from),
    };
    let mut document = load_file(path)?;
    apply_env_overrides(&mut document)?;

    let mut config: Config =
        yaml_serde::from_value(document).map_err(|e| ConfigError::Invalid(e.to_string()))?;
    if let Some(port) = source.port {
        let mut address = config
            .startup
            .bind
            .parse::<std::net::SocketAddr>()
            .map_err(|_| {
                ConfigError::Invalid(format!(
                    "--port cannot be applied: startup.bind '{}' is not an address:port",
                    config.startup.bind
                ))
            })?;
        address.set_port(port);
        config.startup.bind = address.to_string();
    }
    if let Some(dir) = &source.data_dir {
        config.startup.data_dir = dir.clone();
    }
    config.startup.http.auth.api_key = server_api_key()?;
    if let Some(embedding) = config
        .startup
        .embedding
        .as_mut()
        .filter(|embedding| embedding.provider == "openai")
    {
        embedding.api_key = read_env(OPENAI_API_KEY_ENV)?;
    }

    config.validate().map_err(ConfigError::Invalid)?;
    Ok(config)
}

/// Parse the configuration file into an untyped document, or an empty one when there is none.
fn load_file(path: Option<PathBuf>) -> Result<Value, ConfigError> {
    let Some(path) = path else {
        return Ok(Value::Mapping(Mapping::new()));
    };
    let shown = path.display();
    let data = fs::read_to_string(&path)
        .map_err(|e| ConfigError::File(format!("failed to read CONFIG_FILE '{shown}': {e}")))?;

    let parsed = match path.extension().and_then(std::ffi::OsStr::to_str) {
        Some("yaml" | "yml") => yaml_serde::from_str::<Value>(&data)
            .map_err(|e| ConfigError::File(format!("failed to parse YAML '{shown}': {e}")))?,
        Some("json") => serde_json::from_str::<Value>(&data)
            .map_err(|e| ConfigError::File(format!("failed to parse JSON '{shown}': {e}")))?,
        _ => {
            return Err(ConfigError::File(format!(
                "unsupported CONFIG_FILE extension for '{shown}', expected .yaml, .yml, or .json"
            )))
        }
    };

    // An empty file parses as null and takes every default.
    Ok(match parsed {
        Value::Null => Value::Mapping(Mapping::new()),
        other => other,
    })
}

/// Merge every PIRAMID__ variable into the document at the path its name spells out.
fn apply_env_overrides(document: &mut Value) -> Result<(), ConfigError> {
    let mut overrides: Vec<(String, String)> = Vec::new();
    for (name, raw) in env::vars_os() {
        if !name.as_encoded_bytes().starts_with(ENV_PREFIX.as_bytes()) {
            continue;
        }
        let (Some(name_text), Some(raw_text)) = (name.to_str(), raw.to_str()) else {
            return Err(ConfigError::Env {
                name: name.to_string_lossy().into_owned(),
                reason: "not valid UTF-8".to_string(),
            });
        };
        overrides.push((name_text.to_string(), raw_text.to_string()));
    }
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

/// Read an environment variable. None when it is unset, an error when it is not UTF-8.
fn read_env(name: &str) -> Result<Option<String>, ConfigError> {
    match env::var(name) {
        Ok(value) => Ok(Some(value)),
        Err(VarError::NotPresent) => Ok(None),
        Err(VarError::NotUnicode(_)) => Err(ConfigError::Env {
            name: name.to_string(),
            reason: "not valid UTF-8".to_string(),
        }),
    }
}

/// Read the server API key from PIRAMID_API_KEY. None when it is unset.
fn server_api_key() -> Result<Option<ApiKey>, ConfigError> {
    let Some(key) = read_env(API_KEY_ENV)? else {
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
