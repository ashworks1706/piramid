use crate::config::AppConfig;
use std::fs;

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
