use crate::config::AppConfig;
use std::fs;

/// Resolved runtime configuration (application + process-level settings).
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub app: AppConfig,
    pub port: u16,
    pub data_dir: String,
    pub slow_query_ms: u128,
    pub embedding: Option<crate::embeddings::EmbeddingConfig>,
    pub disk_min_free_bytes: Option<u64>,
    pub disk_readonly_on_low_space: bool,
    pub cache_max_bytes: Option<u64>,
}

/// Load configuration from (optional) file, then apply environment overrides.
pub fn load_app_config() -> AppConfig {
    let mut cfg = if let Some(file_cfg) = load_from_file() {
        file_cfg
    } else {
        AppConfig::default()
    };

    // Apply environment overrides
    cfg.apply_env_overrides();

    // Fail fast on invalid settings
    if let Err(e) = cfg.validate() {
        eprintln!("Invalid configuration: {}", e);
        std::process::exit(1);
    }
    // Log resolved configuration for visibility
    println!("Using configuration: {:?}", cfg);
    cfg
}

/// Load everything the server needs (AppConfig + env-driven runtime knobs).
pub fn load_runtime_config() -> RuntimeConfig {
    let app = load_app_config();

    let port = std::env::var("PORT")
        .ok()
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(6333);
    let data_dir = std::env::var("DATA_DIR").unwrap_or_else(|_| "./.piramid".to_string());
    let slow_query_ms = std::env::var("SLOW_QUERY_MS")
        .ok()
        .and_then(|v| v.parse::<u128>().ok())
        .unwrap_or(500);

    let embedding_provider = std::env::var("EMBEDDING_PROVIDER").ok();
    let embedding_model = std::env::var("EMBEDDING_MODEL").ok();
    let embedding_base_url = std::env::var("EMBEDDING_BASE_URL").ok();
    let embedding_api_key = std::env::var("OPENAI_API_KEY").ok();
    let embedding_timeout = std::env::var("EMBEDDING_TIMEOUT_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok());

    let disk_min_free_bytes = std::env::var("DISK_MIN_FREE_BYTES")
        .ok()
        .and_then(|v| v.parse::<u64>().ok());
    let disk_readonly_on_low_space = std::env::var("DISK_READONLY_ON_LOW_SPACE")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(true);
    let cache_max_bytes = std::env::var("CACHE_MAX_BYTES")
        .ok()
        .and_then(|v| v.parse::<u64>().ok());

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
        cache_max_bytes,
    }
}

fn load_from_file() -> Option<AppConfig> {
    let path = std::env::var("CONFIG_FILE").ok()?;
    let data = fs::read_to_string(&path).ok()?;

    if path.ends_with(".yaml") || path.ends_with(".yml") {
        serde_yaml::from_str::<AppConfig>(&data).ok()
    } else if path.ends_with(".json") {
        serde_json::from_str::<AppConfig>(&data).ok()
    } else {
        None
    }
}
