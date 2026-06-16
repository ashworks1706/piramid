use crate::config::AppConfig;
use std::env;
use std::fs;
use std::path::PathBuf;

/// Resolved runtime configuration application + process-level settings for the server.
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub app: AppConfig,
    pub port: u16,
    pub data_dir: String,
    pub slow_query_ms: u128,
    pub embedding: Option<crate::embeddings::EmbeddingConfig>,
    pub disk_min_free_bytes: Option<u64>,
    pub disk_readonly_on_low_space: bool,
}

/// Load configuration from file, then apply environment overrides.
pub fn load_app_config() -> AppConfig {
    let mut cfg = match load_from_file() {
        Ok(Some(cfg)) => cfg,
        Ok(None) => AppConfig::default(),
        Err(e) => {
            eprintln!("Invalid configuration file: {e}");
            std::process::exit(1);
        }
    };

    // Apply environment overrides
    if let Err(e) = cfg.apply_env_overrides() {
        eprintln!("Invalid environment configuration: {e}");
        std::process::exit(1);
    }

    // Fail fast on invalid settings
    if let Err(e) = cfg.validate() {
        eprintln!("Invalid configuration: {}", e);
        std::process::exit(1);
    }
    cfg
}

/// Load everything the server needs (AppConfig + env-driven runtime knobs).
pub fn load_runtime_config() -> RuntimeConfig {
    let app = load_app_config();

    let port = parse_env_or_default("PORT", 6333u16);
    let data_dir = env::var("DATA_DIR").unwrap_or_else(|_| default_data_dir());
    let slow_query_default = app.logging.slow_query_ms.unwrap_or(500) as u128;
    let slow_query_ms = parse_env_or_default("SLOW_QUERY_MS", slow_query_default);

    let embedding_provider = env::var("EMBEDDING_PROVIDER").ok();
    let embedding_model = env::var("EMBEDDING_MODEL").ok();
    let embedding_base_url = env::var("EMBEDDING_BASE_URL").ok();
    let embedding_api_key = env::var("OPENAI_API_KEY").ok();
    let embedding_timeout = parse_optional_env("EMBEDDING_TIMEOUT_SECS");

    let disk_min_free_bytes = parse_optional_env("DISK_MIN_FREE_BYTES");
    let disk_readonly_on_low_space = match env::var("DISK_READONLY_ON_LOW_SPACE") {
        Ok(value) => parse_bool_env("DISK_READONLY_ON_LOW_SPACE", &value).unwrap_or_else(|e| {
            eprintln!("Invalid environment configuration: {e}");
            std::process::exit(1);
        }),
        Err(_) => true,
    };
    let embedding = embedding_provider.map(|provider| {
        let model = embedding_model.unwrap_or_else(|| {
            if provider == "openai" {
                "text-embedding-3-small".to_string()
            } else if provider == "ollama" {
                "nomic-embed-text".to_string()
            } else {
                "text-embedding-3-small".to_string()
            }
        });

        crate::embeddings::EmbeddingConfig {
            provider,
            model,
            api_key: embedding_api_key,
            base_url: embedding_base_url,
            options: serde_json::json!({}),
            timeout: embedding_timeout,
        }
    });

    RuntimeConfig {
        app,
        port,
        data_dir,
        slow_query_ms,
        embedding,
        disk_min_free_bytes,
        disk_readonly_on_low_space,
    }
}

fn load_from_file() -> Result<Option<AppConfig>, String> {
    let path = match env::var("CONFIG_FILE") {
        Ok(path) => path,
        Err(_) => return Ok(None),
    };
    let data = fs::read_to_string(&path)
        .map_err(|e| format!("failed to read CONFIG_FILE '{path}': {e}"))?;

    if path.ends_with(".yaml") || path.ends_with(".yml") {
        serde_yaml::from_str::<AppConfig>(&data)
            .map(Some)
            .map_err(|e| format!("failed to parse YAML CONFIG_FILE '{path}': {e}"))
    } else if path.ends_with(".json") {
        serde_json::from_str::<AppConfig>(&data)
            .map(Some)
            .map_err(|e| format!("failed to parse JSON CONFIG_FILE '{path}': {e}"))
    } else {
        Err(format!(
            "unsupported CONFIG_FILE extension for '{path}', expected .yaml, .yml, or .json"
        ))
    }
}

pub fn default_data_dir() -> String {
    let home = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    let mut path = PathBuf::from(home);
    path.push(".piramid");
    path.to_string_lossy().to_string()
}

fn parse_env_or_default<T>(name: &str, default: T) -> T
where
    T: std::str::FromStr,
{
    match env::var(name) {
        Ok(value) => value.parse::<T>().unwrap_or_else(|_| {
            eprintln!("Invalid environment configuration: Invalid {name} value '{value}'");
            std::process::exit(1);
        }),
        Err(_) => default,
    }
}

fn parse_optional_env<T>(name: &str) -> Option<T>
where
    T: std::str::FromStr,
{
    env::var(name).ok().map(|value| {
        value.parse::<T>().unwrap_or_else(|_| {
            eprintln!("Invalid environment configuration: Invalid {name} value '{value}'");
            std::process::exit(1);
        })
    })
}

fn parse_bool_env(name: &str, value: &str) -> Result<bool, String> {
    match value.to_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        _ => Err(format!("Invalid {name} value '{value}'")),
    }
}
