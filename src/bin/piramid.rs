// Piramid CLI: unified interface for server and setup tasks.
// This binary provides a single entry point for starting the server and performing setup tasks like generating config files. It replaces the previous piramid-server binary and adds new functionality.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use clap::{Parser, Subcommand, ValueEnum};
use piramid::config::{self, AppConfig};
use piramid::cli::animation;
use piramid::server::state::AppState;
use piramid::{config::loader::RuntimeConfig, embeddings, server};
use tokio::runtime::Runtime;

/// Unified CLI for Piramid (server + setup helpers).
#[derive(Parser)]
#[command(author, version, about = "Piramid CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the server directly (replaces piramid-server).
    Serve {
        /// Optional config file (sets CONFIG_FILE)
        #[arg(long)]
        config: Option<PathBuf>,
        /// Override port (sets PORT)
        #[arg(long)]
        port: Option<u16>,
        /// Override data dir (sets DATA_DIR)
        #[arg(long)]
        data_dir: Option<PathBuf>,
        /// Skip the short animation
        #[arg(long)]
        no_anim: bool,
    },

    /// Generate a config file with defaults (YAML).
    Init {
        /// Path to write the config file (default: piramid.yaml)
        #[arg(long, short, default_value = "piramid.yaml")]
        path: PathBuf,
        /// Output format (yaml or json)
        #[arg(long, value_enum, default_value_t = OutputFormat::Yaml)]
        format: OutputFormat,
        /// Skip the short animation
        #[arg(long)]
        no_anim: bool,
    },

    /// Show the resolved config (after env overrides).
    ShowConfig {
        /// Optional config file to load (overrides CONFIG_FILE)
        #[arg(long)]
        config: Option<PathBuf>,
    },
}

#[derive(Copy, Clone, ValueEnum)]
enum OutputFormat {
    Yaml,
    Json,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Init { path, format, no_anim } => {
            if !no_anim {
                animate("Generating config");
            }
            if let Err(e) = write_config_file(&path, format) {
                eprintln!("Failed to write config: {e}");
                std::process::exit(1);
            }
            println!("Wrote config to {}", path.display());
        }
        Commands::ShowConfig { config } => {
            if let Some(path) = config {
                std::env::set_var("CONFIG_FILE", path);
            }
            let cfg = config::loader::load_app_config();
            let yaml = serde_yaml::to_string(&cfg).unwrap_or_else(|_| format!("{cfg:?}"));
            println!("{yaml}");
        }
        Commands::Serve {
            config,
            port,
            data_dir,
            no_anim,
        } => {
            if let Some(path) = config {
                std::env::set_var("CONFIG_FILE", path);
            }
            if let Some(port) = port {
                std::env::set_var("PORT", port.to_string());
            }
            if let Some(dir) = data_dir {
                std::env::set_var("DATA_DIR", dir);
            }
            if !no_anim {
                animate("Starting piramid-server");
            }
            if let Err(e) = start_server_inline() {
                eprintln!("Failed to start piramid-server: {e}");
                std::process::exit(1);
            }
        }
    }
}

fn write_config_file(path: &Path, fmt: OutputFormat) -> std::io::Result<()> {
    let cfg = AppConfig::default();
    let contents = match fmt {
        OutputFormat::Yaml => serde_yaml::to_string(&cfg).unwrap_or_default(),
        OutputFormat::Json => serde_json::to_string_pretty(&cfg).unwrap_or_default(),
    };
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    fs::write(path, contents)
}

fn start_server_inline() -> std::io::Result<()> {
    let rt = Runtime::new().map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    rt.block_on(async {
        let RuntimeConfig {
            app: app_config,
            port,
            data_dir,
            slow_query_ms,
            embedding: embedding_config,
            disk_min_free_bytes,
            disk_readonly_on_low_space,
            cache_max_bytes,
        } = piramid::config::loader::load_runtime_config();

        let state = match embedding_config.clone() {
            Some(config) => {
                let timeout = std::env::var("EMBEDDING_TIMEOUT_SECS")
                    .ok()
                    .and_then(|s| s.parse::<u64>().ok());
                let mut config = config;
                if config.timeout.is_none() {
                    config.timeout = timeout;
                }
                match embeddings::providers::create_embedder(&config) {
                    Ok(embedder) => {
                        let retry_embedder = std::sync::Arc::new(embeddings::RetryEmbedder::new(embedder));
                        std::sync::Arc::new(AppState::with_embedder(
                            &data_dir,
                            app_config.clone(),
                            slow_query_ms,
                            retry_embedder,
                            disk_min_free_bytes,
                            disk_readonly_on_low_space,
                            cache_max_bytes,
                        ))
                    }
                    Err(_) => std::sync::Arc::new(AppState::new(
                        &data_dir,
                        app_config.clone(),
                        slow_query_ms,
                        disk_min_free_bytes,
                        disk_readonly_on_low_space,
                        cache_max_bytes,
                    )),
                }
            }
            None => std::sync::Arc::new(AppState::new(
                &data_dir,
                app_config.clone(),
                slow_query_ms,
                disk_min_free_bytes,
                disk_readonly_on_low_space,
                cache_max_bytes,
            )),
        };

        let app = server::create_router(state);
        let addr = format!("0.0.0.0:{}", port);
        let listener = tokio::net::TcpListener::bind(&addr).await.map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::Other, format!("bind failed: {e}"))
        })?;
        axum::serve(listener, app)
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    })
}

fn animate(label: &str) {
    for (i, frame) in animation::CLI_FRAMES.iter().enumerate() {
        print!("\r{label}\n{frame}");
        let _ = std::io::stdout().flush();
        thread::sleep(Duration::from_millis(60));
        if i > 12 {
            break;
        }
    }
    println!("\r{label} ✓");
}
