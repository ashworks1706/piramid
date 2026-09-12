//! The piramid binary: subcommand parsing, server startup, and the developer console.

use std::io::Write;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use clap::{Parser, Subcommand};
mod animation;
mod console;
mod support;
use piramid::config::StartupConfig;
use piramid::observability;
use piramid::state::AppState;
use piramid::{embeddings, server};
use tokio::runtime::Runtime;

#[derive(Parser)]
#[command(author, version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the server in the foreground, with no terminal UI
    Serve {
        /// Config file to load
        #[arg(long)]
        config: Option<PathBuf>,
        /// Port to bind
        #[arg(long)]
        port: Option<u16>,
        /// Data directory
        #[arg(long)]
        data_dir: Option<PathBuf>,
    },

    /// Write a diagnostic bundle to attach to a bug report. Secrets are redacted, review it first
    SupportBundle {
        /// Where to write the bundle
        #[arg(long, short, default_value = "piramid-support-bundle.md")]
        output: PathBuf,
        /// Config file to load
        #[arg(long)]
        config: Option<PathBuf>,
        /// Data directory to inspect
        #[arg(long)]
        data_dir: Option<PathBuf>,
    },
}

/// Run action, printing context and exiting 1 on failure.
fn run_or_exit(action: impl FnOnce() -> std::io::Result<()>, context: &str) {
    if let Err(e) = action() {
        eprintln!("{context}: {e}");
        std::process::exit(1);
    }
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Some(Commands::SupportBundle {
            output,
            config,
            data_dir,
        }) => {
            run_or_exit(
                || support_bundle(output, config, data_dir),
                "Failed to write support bundle",
            );
        }
        Some(Commands::Serve {
            config,
            port,
            data_dir,
        }) => {
            animate();
            let config = load_config(config.as_deref(), port, data_dir.as_deref())
                .unwrap_or_else(exit_on_config_error);
            run_or_exit(|| start_server_inline(config), "Failed to start the server");
        }
        // No subcommand opens the console. Inside a checkout it can drive the repo as well as
        // the server; an installed binary gets the views that need only a server.
        None => {
            let config = piramid::config::loader::load().unwrap_or_else(exit_on_config_error);
            let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            let (profile, root) = match console::repo_root(&cwd) {
                Some(root) => (console::Profile::Developer, root),
                None => (console::Profile::Production, cwd),
            };
            run_or_exit(
                || console::run(&config, profile, root),
                "Failed to start the console",
            );
        }
    }
}

/// Report a configuration failure and exit.
fn exit_on_config_error<T>(error: piramid::error::ConfigError) -> T {
    eprintln!("piramid: {error}");
    std::process::exit(1);
}

/// Load the configuration with the command-line flags applied over the file and environment.
///
/// A port replaces the port of startup.bind and keeps its host. A data directory replaces
/// startup.data_dir.
fn load_config(
    file: Option<&std::path::Path>,
    port: Option<u16>,
    data_dir: Option<&std::path::Path>,
) -> Result<piramid::config::Config, piramid::error::ConfigError> {
    let mut unparsable_bind = None;
    let config = piramid::config::loader::load_with(file, |config| {
        if let Some(port) = port {
            match config.startup.bind.parse::<std::net::SocketAddr>() {
                Ok(mut address) => {
                    address.set_port(port);
                    config.startup.bind = address.to_string();
                }
                Err(_) => unparsable_bind = Some(config.startup.bind.clone()),
            }
        }
        if let Some(dir) = data_dir {
            config.startup.data_dir = dir.to_string_lossy().into_owned();
        }
    })?;
    if let Some(bind) = unparsable_bind {
        return Err(piramid::error::ConfigError::Invalid(format!(
            "--port cannot be applied: startup.bind '{bind}' is not an address:port"
        )));
    }
    Ok(config)
}

fn support_bundle(
    output: PathBuf,
    config: Option<PathBuf>,
    data_dir: Option<PathBuf>,
) -> std::io::Result<()> {
    let config = load_config(config.as_deref(), None, data_dir.as_deref())
        .unwrap_or_else(exit_on_config_error);
    let state = std::sync::Arc::new(
        AppState::new(config.clone(), embeddings::EmbeddingsManager::disabled())
            .map_err(std::io::Error::other)?,
    );
    preload_collections_for_metrics(&state)?;

    let path = support::write(&config, &state, Some(output))?;
    println!("wrote {}", path.display());
    println!("Review it before sharing — it contains your configuration and collection names.");
    Ok(())
}

/// Open every collection on disk so the bundle reports it, naming each one that fails to open.
fn preload_collections_for_metrics(state: &std::sync::Arc<AppState>) -> std::io::Result<()> {
    let names = state
        .collection_manager
        .discover_on_disk()
        .map_err(std::io::Error::other)?;
    for collection_name in names {
        if let Err(error) = state.get_existing_collection(&collection_name) {
            eprintln!(
                "Collection '{collection_name}' failed to open and is not in the bundle: {error}"
            );
        }
    }
    Ok(())
}

fn start_server_inline(config: piramid::config::Config) -> std::io::Result<()> {
    let rt = Runtime::new().map_err(std::io::Error::other)?;
    rt.block_on(async {
        let _observability =
            observability::install(config.startup.logging, &config.startup.telemetry);
        init_thread_pool(&config.startup);
        if config.startup.logging.config {
            tracing::info!(
                target: "piramid::config",
                config = ?config,
                "using_configuration"
            );
        }

        let embeddings = match &config.startup.embedding {
            Some(embedding) => {
                embeddings::EmbeddingsManager::from_config(embedding).map_err(|e| {
                    std::io::Error::other(format!(
                        "embedding provider configured but failed to initialize: {e}"
                    ))
                })?
            }
            None => embeddings::EmbeddingsManager::disabled(),
        };
        let addr = config.startup.bind.clone();
        let data_dir = config.startup.data_dir.clone();
        let state =
            std::sync::Arc::new(AppState::new(config, embeddings).map_err(std::io::Error::other)?);

        let app = server::create_router(state);
        tracing::info!(
            target: "piramid::config",
            address = addr.as_str(),
            data_dir = data_dir.as_str(),
            "server_starting"
        );
        let listener = tokio::net::TcpListener::bind(&addr)
            .await
            .map_err(|e| std::io::Error::other(format!("bind failed: {e}")))?;
        axum::serve(listener, app)
            .await
            .map_err(std::io::Error::other)
    })
}

/// Build the global rayon pool. Called once, before any collection opens.
fn init_thread_pool(startup: &StartupConfig) {
    let num_threads = startup.num_threads();
    if let Err(error) = rayon::ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .build_global()
    {
        tracing::warn!(target: "piramid::config", %error, "thread_pool_already_built");
    }
}

fn animate() {
    // The splash is skipped when stdout is not a terminal.
    if !std::io::IsTerminal::is_terminal(&std::io::stdout()) {
        return;
    }
    print!("\x1b[2J\x1b[H\x1b[?25l");
    let _ = std::io::stdout().flush();

    for (i, frame) in animation::CLI_FRAMES.iter().enumerate() {
        print!("\x1b[H{frame}");
        let _ = std::io::stdout().flush();
        thread::sleep(Duration::from_millis(45));
        if i > 12 {
            break;
        }
    }

    print!("\x1b[2J\x1b[H\n\x1b[?25h");
    let _ = std::io::stdout().flush();
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "a failed assertion is the point of a test"
)]
mod tests {
    use super::*;

    fn file(name: &str, contents: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("piramid-cli-{}-{name}", std::process::id()));
        std::fs::write(&path, contents).unwrap();
        path
    }

    #[test]
    fn serve_flags_replace_the_port_and_the_data_directory() {
        let path = file(
            "flags.yaml",
            "startup:\n  bind: 127.0.0.1:6333\n  data_dir: ./from-file\n",
        );
        let config = load_config(
            Some(&path),
            Some(7000),
            Some(std::path::Path::new("/tmp/elsewhere")),
        )
        .unwrap();
        assert_eq!(config.startup.bind, "127.0.0.1:7000");
        assert_eq!(config.startup.data_dir, "/tmp/elsewhere");
    }

    #[test]
    fn no_flags_keep_what_the_file_says() {
        let path = file(
            "noflags.yaml",
            "startup:\n  bind: 127.0.0.1:6333\n  data_dir: ./from-file\n",
        );
        let config = load_config(Some(&path), None, None).unwrap();
        assert_eq!(config.startup.bind, "127.0.0.1:6333");
        assert_eq!(config.startup.data_dir, "./from-file");
    }
}
